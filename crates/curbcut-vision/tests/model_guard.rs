//! Integration test for the model guard.
//!
//! Every scenario a caller can meet: a model that verifies, a model that was
//! tampered with, a truncated file, a missing file, a forged signature, an
//! untrusted key, a build with no backend, and a malformed pin. The table
//! printed under `--nocapture` is the triage view. See
//! [`../../../docs/MODEL_GUARD.md`](../../../docs/MODEL_GUARD.md).

use curbcut_core::hash::sha256;
use curbcut_vision::model_guard::{
    load, HmacReleaseKey, LoadOutcome, Manifest, ModelSpec, UnsupportedVerifier, VerificationError,
};

/// A stand-in artifact. It is not a real model; the guard does not care.
fn artifact() -> Vec<u8> {
    b"stand-in-model-artifact".to_vec()
}

/// A correctly pinned and signed model.
fn pinned() -> (ModelSpec, HmacReleaseKey, Vec<u8>) {
    let bytes = artifact();
    let key = HmacReleaseKey::new("release-2026-01", b"release-secret");
    let mut spec = ModelSpec::new(
        "det-v2",
        "1.0.0",
        sha256(&bytes).as_hex(),
        bytes.len() as u64,
        String::new(),
        "release-2026-01",
    );
    spec.signature = key.sign(&spec.signature_message());
    (spec, key, bytes)
}

/// The triage row for one outcome.
fn classify(outcome: &LoadOutcome<'_>) -> (&'static str, &'static str) {
    match outcome {
        LoadOutcome::Loaded(_) => ("loaded", "full"),
        LoadOutcome::Missing => ("missing", "rules-only"),
        LoadOutcome::Rejected(error) => (reason(*error), "rules-only"),
    }
}

/// The content-free reason slug.
fn reason(error: VerificationError) -> &'static str {
    match error {
        VerificationError::HashMismatch => "hash-mismatch",
        VerificationError::SignatureMismatch => "signature-mismatch",
        VerificationError::Unsupported => "unsupported",
        VerificationError::UnknownKey => "unknown-key",
        VerificationError::MalformedPin => "malformed-pin",
    }
}

#[test]
fn every_scenario_is_triaged() {
    let (spec, key, bytes) = pinned();
    let mut tampered = bytes.clone();
    tampered[0] ^= 0xff;
    let truncated = bytes[..bytes.len() - 1].to_vec();
    let mut forged = spec.clone();
    forged.signature = key.sign("a different message");
    let mut malformed = spec.clone();
    malformed.sha256 = "not-a-digest".to_string();
    let other_key = HmacReleaseKey::new("release-2027-01", b"release-secret");

    let scenarios: Vec<(&str, LoadOutcome<'_>)> = vec![
        ("pinned", load(&spec, Some(&bytes), &key)),
        ("tampered", load(&spec, Some(&tampered), &key)),
        ("truncated", load(&spec, Some(&truncated), &key)),
        ("missing", load(&spec, None, &key)),
        ("forged-signature", load(&forged, Some(&bytes), &key)),
        ("unknown-key", load(&spec, Some(&bytes), &other_key)),
        (
            "no-backend",
            load(&spec, Some(&bytes), &UnsupportedVerifier),
        ),
        ("malformed-pin", load(&malformed, Some(&bytes), &key)),
    ];

    println!("model guard triage");
    println!("{:<18} {:<18} {:<14}", "scenario", "outcome", "fallback");
    for (name, outcome) in &scenarios {
        let (outcome_slug, fallback) = classify(outcome);
        println!("{name:<18} {outcome_slug:<18} {fallback:<14}");
    }

    let expected = vec![
        ("pinned", "loaded"),
        ("tampered", "hash-mismatch"),
        ("truncated", "hash-mismatch"),
        ("missing", "missing"),
        ("forged-signature", "signature-mismatch"),
        ("unknown-key", "unknown-key"),
        ("no-backend", "unsupported"),
        ("malformed-pin", "malformed-pin"),
    ];
    let actual: Vec<(&str, &str)> = scenarios
        .iter()
        .map(|(name, outcome)| (*name, classify(outcome).0))
        .collect();
    assert_eq!(
        actual, expected,
        "each scenario must be triaged as expected"
    );
}

#[test]
fn only_a_verified_model_yields_bytes() {
    let (spec, key, bytes) = pinned();
    let mut tampered = bytes.clone();
    tampered[0] ^= 0xff;

    let loaded = load(&spec, Some(&bytes), &key);
    assert!(loaded.is_loaded());
    assert_eq!(loaded.fallback().as_str(), "full");

    let bad = [
        load(&spec, Some(&tampered), &key),
        load(&spec, None, &key),
        load(&spec, Some(&bytes), &UnsupportedVerifier),
    ];
    for outcome in &bad {
        assert!(
            !matches!(outcome, LoadOutcome::Loaded(_)),
            "a bad model must never yield bytes"
        );
        assert!(!outcome.is_loaded());
        assert_eq!(outcome.fallback().as_str(), "rules-only");
    }
}

#[test]
fn a_manifest_only_loads_the_version_it_pins() {
    let (spec, key, bytes) = pinned();
    let manifest = Manifest::new(vec![spec.clone()]);
    let found = manifest
        .find("det-v2", "1.0.0")
        .expect("the pin is present");
    assert!(load(found, Some(&bytes), &key).is_loaded());
    assert!(manifest.find("det-v2", "0.9.0").is_none());
    assert!(manifest.find("ocr-v6", "1.0.0").is_none());
}
