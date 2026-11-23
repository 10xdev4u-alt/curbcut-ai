//! Model hash pinning and signature verification.
//!
//! A model is not loaded on trust. [`verify_hash`] checks the bytes against the
//! pinned SHA-256 digest, and a [`Verifier`] checks the release signature over
//! the pin. A model that fails either check is never handed to an inference
//! backend: [`load`] returns [`LoadOutcome::Rejected`], and the caller falls
//! back down the chain in `docs/MODELS.md` §5.
//!
//! The pin is a hash, not content: a model file is never logged, and an error
//! names a reason, never a byte. See
//! [`../../../docs/MODEL_GUARD.md`](../../../docs/MODEL_GUARD.md).

use std::fmt;

use curbcut_core::hash::{hmac_sha256, sha256};

/// Why a model pin or signature did not verify.
///
/// The variants carry no detail: an error must not quote a digest, a key, or a
/// model byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerificationError {
    /// The bytes do not hash to the pinned digest.
    HashMismatch,
    /// The signature did not verify.
    SignatureMismatch,
    /// This build has no signature backend.
    Unsupported,
    /// The pinned key id is not known to the verifier.
    UnknownKey,
    /// The pinned digest is not 64 hexadecimal characters.
    MalformedPin,
}

impl VerificationError {
    /// The content-free reason, safe for a log or a UI message.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HashMismatch => "the model does not match its pinned hash",
            Self::SignatureMismatch => "the model signature did not verify",
            Self::Unsupported => "no signature backend is available",
            Self::UnknownKey => "the pinned signing key is not trusted",
            Self::MalformedPin => "the pinned hash is malformed",
        }
    }
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for VerificationError {}

/// A pinned model artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSpec {
    /// Model id, such as `det-v2`.
    pub id: String,
    /// Model version, such as `1.0.0`.
    pub version: String,
    /// Pinned SHA-256 digest, lowercase hexadecimal.
    pub sha256: String,
    /// Expected size in bytes.
    pub size_bytes: u64,
    /// Release signature over [`ModelSpec::signature_message`].
    pub signature: String,
    /// Id of the key that produced the signature.
    pub key_id: String,
}

impl ModelSpec {
    /// Creates a pin.
    pub fn new(
        id: &str,
        version: &str,
        sha256: impl Into<String>,
        size_bytes: u64,
        signature: impl Into<String>,
        key_id: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            version: version.to_string(),
            sha256: sha256.into(),
            size_bytes,
            signature: signature.into(),
            key_id: key_id.to_string(),
        }
    }

    /// The message a release signature covers.
    ///
    /// It binds the id, the version, the pinned digest, and the size, so a
    /// signature cannot be replayed onto a different pin.
    pub fn signature_message(&self) -> String {
        format!(
            "{}\n{}\n{}\n{}",
            self.id, self.version, self.sha256, self.size_bytes
        )
    }
}

/// The set of pinned models, read from `models/manifest.json` by the updater.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Manifest {
    models: Vec<ModelSpec>,
}

impl Manifest {
    /// Creates a manifest.
    pub fn new(models: Vec<ModelSpec>) -> Self {
        Self { models }
    }

    /// Finds a model by id and version.
    pub fn find(&self, id: &str, version: &str) -> Option<&ModelSpec> {
        self.models
            .iter()
            .find(|model| model.id == id && model.version == version)
    }

    /// The number of pinned models.
    pub fn len(&self) -> usize {
        self.models.len()
    }

    /// Whether the manifest is empty.
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }
}

/// A signature backend.
///
/// Production supplies an Ed25519 verifier backed by the release public key;
/// [`HmacReleaseKey`] is a symmetric release MAC for builds and tests, and
/// [`UnsupportedVerifier`] is the fail-closed default.
pub trait Verifier {
    /// Verifies `signature` over `message` under `key_id`.
    fn verify(
        &self,
        key_id: &str,
        message: &[u8],
        signature: &str,
    ) -> Result<(), VerificationError>;
}

/// A symmetric release key.
///
/// The key is a release secret, not a per-user secret: it authenticates that a
/// model artifact came from the release pipeline. It is held by the updater and
/// never leaves the device in a user-visible form. Rotate by adding a new
/// `key_id` and re-signing the manifest.
pub struct HmacReleaseKey {
    key_id: String,
    key: Vec<u8>,
}

impl fmt::Debug for HmacReleaseKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HmacReleaseKey")
            .field("key_id", &self.key_id)
            .field("key_len", &self.key.len())
            .finish_non_exhaustive()
    }
}

impl HmacReleaseKey {
    /// Creates a release key.
    pub fn new(key_id: &str, key: impl Into<Vec<u8>>) -> Self {
        Self {
            key_id: key_id.to_string(),
            key: key.into(),
        }
    }

    /// Signs a message, for producing a manifest in tests and tooling.
    pub fn sign(&self, message: &str) -> String {
        hmac_sha256(&self.key, message.as_bytes()).as_hex()
    }
}

impl Verifier for HmacReleaseKey {
    fn verify(
        &self,
        key_id: &str,
        message: &[u8],
        signature: &str,
    ) -> Result<(), VerificationError> {
        if key_id != self.key_id {
            return Err(VerificationError::UnknownKey);
        }
        let expected = hmac_sha256(&self.key, message);
        if expected.matches_hex(signature) {
            Ok(())
        } else {
            Err(VerificationError::SignatureMismatch)
        }
    }
}

/// A verifier with no backend.
///
/// It refuses every model. A build that cannot verify must not load a model it
/// cannot authenticate.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnsupportedVerifier;

impl Verifier for UnsupportedVerifier {
    fn verify(
        &self,
        _key_id: &str,
        _message: &[u8],
        _signature: &str,
    ) -> Result<(), VerificationError> {
        Err(VerificationError::Unsupported)
    }
}

/// Checks a model's bytes against its pinned digest.
pub fn verify_hash(spec: &ModelSpec, bytes: &[u8]) -> Result<(), VerificationError> {
    let expected = curbcut_core::hash::Digest::from_hex(&spec.sha256)
        .ok_or(VerificationError::MalformedPin)?;
    if bytes.len() as u64 != spec.size_bytes {
        return Err(VerificationError::HashMismatch);
    }
    if sha256(bytes) == expected {
        Ok(())
    } else {
        Err(VerificationError::HashMismatch)
    }
}

/// Verifies a pinned model and says whether it may be loaded.
///
/// `bytes` is `None` when the model file is absent: that is
/// [`LoadOutcome::Missing`], not a rejection, so the caller can fall back to a
/// smaller model rather than to nothing.
pub fn load<'a>(
    spec: &ModelSpec,
    bytes: Option<&'a [u8]>,
    verifier: &dyn Verifier,
) -> LoadOutcome<'a> {
    let bytes = match bytes {
        Some(bytes) => bytes,
        None => return LoadOutcome::Missing,
    };
    if let Err(error) = verify_hash(spec, bytes) {
        return LoadOutcome::Rejected(error);
    }
    if let Err(error) = verifier.verify(
        &spec.key_id,
        spec.signature_message().as_bytes(),
        &spec.signature,
    ) {
        return LoadOutcome::Rejected(error);
    }
    LoadOutcome::Loaded(bytes)
}

/// The result of a guarded load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOutcome<'a> {
    /// The model verified; the bytes may be handed to a backend.
    Loaded(&'a [u8]),
    /// No model file was present.
    Missing,
    /// The model was present but not usable. Fall back, never load it.
    Rejected(VerificationError),
}

impl LoadOutcome<'_> {
    /// Whether the model may be loaded.
    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded(_))
    }

    /// The next rung of the fallback chain, if any.
    ///
    /// The chain is in `docs/MODELS.md` §5: full pipeline, rules-only,
    /// a11y-only, static overlay. A rejection drops one rung; a missing model
    /// drops to the same rung as a rejection, because both mean "no verified
    /// model available".
    pub fn fallback(&self) -> Fallback {
        match self {
            Self::Loaded(_) => Fallback::FullPipeline,
            Self::Missing | Self::Rejected(_) => Fallback::RulesOnly,
        }
    }
}

/// A rung of the model fallback chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fallback {
    /// Detector, OCR, caption, and simplification.
    FullPipeline,
    /// Detector and OCR with heuristics; no large model.
    RulesOnly,
    /// Accessibility tree and heuristics; no vision.
    A11yOnly,
    /// The saved profile and manual tools.
    StaticOverlay,
}

impl Fallback {
    /// The content-free slug, safe for a UI message and a log.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FullPipeline => "full",
            Self::RulesOnly => "rules-only",
            Self::A11yOnly => "a11y-only",
            Self::StaticOverlay => "static-overlay",
        }
    }
}

impl fmt::Display for Fallback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes() -> Vec<u8> {
        b"onnx-model-bytes".to_vec()
    }

    fn spec_and_key() -> (ModelSpec, HmacReleaseKey) {
        let bytes = bytes();
        let digest = sha256(&bytes).as_hex();
        let key = HmacReleaseKey::new("release-2026-01", b"release-secret");
        let mut spec = ModelSpec::new(
            "det-v2",
            "1.0.0",
            digest,
            bytes.len() as u64,
            String::new(),
            "release-2026-01",
        );
        spec.signature = key.sign(&spec.signature_message());
        (spec, key)
    }

    #[test]
    fn a_pinned_model_loads() {
        let (spec, key) = spec_and_key();
        assert_eq!(
            load(&spec, Some(&bytes()), &key),
            LoadOutcome::Loaded(&bytes())
        );
    }

    #[test]
    fn a_tampered_model_is_rejected() {
        let (spec, key) = spec_and_key();
        let mut tampered = bytes();
        tampered[0] ^= 0xff;
        assert_eq!(
            load(&spec, Some(&tampered), &key),
            LoadOutcome::Rejected(VerificationError::HashMismatch)
        );
    }

    #[test]
    fn a_truncated_model_is_rejected() {
        let (spec, key) = spec_and_key();
        let truncated = &bytes()[..bytes().len() - 1];
        assert_eq!(
            load(&spec, Some(truncated), &key),
            LoadOutcome::Rejected(VerificationError::HashMismatch)
        );
    }

    #[test]
    fn a_missing_model_is_missing_not_rejected() {
        let (spec, key) = spec_and_key();
        assert_eq!(load(&spec, None, &key), LoadOutcome::Missing);
        assert_eq!(load(&spec, None, &key).fallback(), Fallback::RulesOnly);
    }

    #[test]
    fn a_forged_signature_is_rejected() {
        let (spec, key) = spec_and_key();
        let mut forged = spec.clone();
        forged.signature = key.sign("a different message");
        assert_eq!(
            load(&forged, Some(&bytes()), &key),
            LoadOutcome::Rejected(VerificationError::SignatureMismatch)
        );
    }

    #[test]
    fn an_unknown_key_is_rejected() {
        let (spec, _) = spec_and_key();
        let other = HmacReleaseKey::new("release-2027-01", b"release-secret");
        assert_eq!(
            load(&spec, Some(&bytes()), &other),
            LoadOutcome::Rejected(VerificationError::UnknownKey)
        );
    }

    #[test]
    fn a_build_without_a_backend_refuses_every_model() {
        let (spec, _) = spec_and_key();
        assert_eq!(
            load(&spec, Some(&bytes()), &UnsupportedVerifier),
            LoadOutcome::Rejected(VerificationError::Unsupported)
        );
    }

    #[test]
    fn a_malformed_pin_is_rejected() {
        let (mut spec, key) = spec_and_key();
        spec.sha256 = "not-a-digest".to_string();
        assert_eq!(
            load(&spec, Some(&bytes()), &key),
            LoadOutcome::Rejected(VerificationError::MalformedPin)
        );
    }

    #[test]
    fn an_uppercase_signature_still_verifies() {
        let (mut spec, key) = spec_and_key();
        spec.signature = spec.signature.to_uppercase();
        assert_eq!(
            load(&spec, Some(&bytes()), &key),
            LoadOutcome::Loaded(&bytes())
        );
    }

    #[test]
    fn a_signature_does_not_replay_onto_another_pin() {
        let (spec, key) = spec_and_key();
        let mut other = spec.clone();
        other.version = "1.0.1".to_string();
        assert_eq!(
            load(&other, Some(&bytes()), &key),
            LoadOutcome::Rejected(VerificationError::SignatureMismatch)
        );
    }

    #[test]
    fn the_manifest_finds_a_model_by_id_and_version() {
        let (spec, _) = spec_and_key();
        let manifest = Manifest::new(vec![spec.clone()]);
        assert_eq!(manifest.len(), 1);
        assert!(!manifest.is_empty());
        assert_eq!(manifest.find("det-v2", "1.0.0"), Some(&spec));
        assert!(manifest.find("det-v2", "1.0.1").is_none());
        assert!(manifest.find("ocr-v6", "1.0.0").is_none());
        assert!(Manifest::default().is_empty());
    }

    #[test]
    fn errors_and_fallbacks_are_content_free() {
        for error in [
            VerificationError::HashMismatch,
            VerificationError::SignatureMismatch,
            VerificationError::Unsupported,
            VerificationError::UnknownKey,
            VerificationError::MalformedPin,
        ] {
            let text = format!("{error}");
            assert!(!text.contains("onnx-model-bytes"));
            assert!(!text.is_empty());
        }
        for fallback in [
            Fallback::FullPipeline,
            Fallback::RulesOnly,
            Fallback::A11yOnly,
            Fallback::StaticOverlay,
        ] {
            assert!(!fallback.as_str().is_empty());
        }
    }

    #[test]
    fn the_documented_rules_match_the_docs() {
        let doc = include_str!("../../../docs/MODEL_GUARD.md");
        for word in [
            "Verifier",
            "ModelSpec",
            "fallback",
            "key_id",
            "models/manifest.json",
        ] {
            assert!(
                doc.contains(word),
                "docs/MODEL_GUARD.md does not state `{word}`"
            );
        }
    }
}
