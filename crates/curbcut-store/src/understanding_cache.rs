//! Understanding-result cache, sealed at rest.
//!
//! [`crate::understanding_cache::UnderstandingCache`] keeps encoded
//! understanding results keyed by content hash, model version, and profile,
//! bounded by a least-recently-used capacity, and stores every value as
//! ciphertext produced by a [`crate::understanding_cache::Cipher`]. See
//! [`../../../docs/UNDERSTANDING_CACHE.md`](../../../docs/UNDERSTANDING_CACHE.md).
//!
//! Three rules hold:
//!
//! - **A hit is a function of the key.** The key is a content hash, a model
//!   version, and a profile. Nothing else — not the app, not the window, not
//!   the time — affects whether a key matches, so the same key under the same
//!   operations always returns the same value.
//! - **Plaintext is never stored.** [`crate::understanding_cache::UnderstandingCache::store`]
//!   seals the payload before it reaches the LRU, and the only way back is
//!   [`crate::understanding_cache::UnderstandingCache::load`], which opens it.
//!   The cache holds ciphertext and
//!   [`crate::understanding_cache::UnderstandingCache::sealed`] hands out
//!   ciphertext for writing to disk.
//! - **Eviction is recency only.** There is no time-based expiry, so eviction
//!   is deterministic under a given operation sequence.
//!
//! The crate ships **no** [`crate::understanding_cache::Cipher`] implementation:
//! the key lives in the OS keychain (macOS Keychain, Windows DPAPI, Linux
//! libsecret), which is a platform adapter concern. A caller without a keychain
//! must not build a shipping profile, and the type system does not pretend
//! otherwise. See `CC-093`.
//!
//! The caller owns the payload encoding and the privacy boundary: a payload may
//! be derived text, so it must never be logged, and neither the key nor the
//! ciphertext may be written to a log.

use std::fmt;

use curbcut_core::id::ProfileId;

use crate::lru::LruCache;
use crate::plan_cache::ContentHash;

/// A model version, used as part of a cache key.
///
/// A result computed by one model version is not valid for another, so the
/// version is part of the key rather than a property of the cache. The version
/// is a model release identifier, never content.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModelVersion(String);

impl ModelVersion {
    /// Creates a version from its string form.
    pub fn new(version: &str) -> Self {
        Self(version.to_string())
    }

    /// The string form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ModelVersion {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// The cache key: a content hash, a model version, and a profile.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnderstandingKey {
    content: ContentHash,
    model: ModelVersion,
    profile: ProfileId,
}

impl UnderstandingKey {
    /// Creates a key.
    pub fn new(content: ContentHash, model: ModelVersion, profile: ProfileId) -> Self {
        Self {
            content,
            model,
            profile,
        }
    }

    /// The content hash.
    pub const fn content(&self) -> ContentHash {
        self.content
    }

    /// The model version.
    pub const fn model(&self) -> &ModelVersion {
        &self.model
    }

    /// The profile.
    pub const fn profile(&self) -> &ProfileId {
        &self.profile
    }
}

/// Why a seal or an open failed.
///
/// The variants carry no detail: an error must not quote a key, a payload, or
/// a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CipherError {
    /// The key is unavailable, so nothing can be sealed or opened.
    NoKey,
    /// Sealing failed.
    SealFailed,
    /// The sealed bytes are malformed or failed to authenticate.
    Corrupt,
}

impl fmt::Display for CipherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::NoKey => "no key available",
            Self::SealFailed => "sealing failed",
            Self::Corrupt => "sealed bytes are corrupt",
        };
        f.write_str(text)
    }
}

impl std::error::Error for CipherError {}

/// A sealer and opener for cache payloads.
///
/// The implementation is supplied by the platform adapter, which holds the
/// keychain key. The cache calls [`Cipher::seal`] before storing and
/// [`Cipher::open`] after a hit, and never sees the key.
pub trait Cipher {
    /// Seals plaintext for storage.
    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, CipherError>;

    /// Opens sealed bytes.
    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, CipherError>;
}

/// A bounded, sealed cache of encoded understanding results.
///
/// The stored value is the ciphertext of the payload, keyed by
/// [`UnderstandingKey`]. Capacity is a number of entries, not bytes, matching
/// [`crate::plan_cache::PlanCache`].
#[derive(Debug, Clone)]
pub struct UnderstandingCache<C: Cipher> {
    inner: LruCache<UnderstandingKey, Vec<u8>>,
    cipher: C,
}

impl<C: Cipher> UnderstandingCache<C> {
    /// Creates a cache that holds at most `capacity` sealed entries.
    ///
    /// A capacity of zero stores nothing.
    pub fn new(capacity: usize, cipher: C) -> Self {
        Self {
            inner: LruCache::new(capacity),
            cipher,
        }
    }

    /// The cipher the cache seals and opens with.
    pub const fn cipher(&self) -> &C {
        &self.cipher
    }

    /// The maximum number of entries.
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// The number of sealed entries.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Seals and stores a payload.
    ///
    /// Storing an existing key replaces it and marks it most recently used,
    /// without growing the cache. A capacity of zero stores nothing and still
    /// seals, so a caller cannot infer the capacity from the cipher's workload.
    pub fn store(&mut self, key: UnderstandingKey, payload: &[u8]) -> Result<(), CipherError> {
        let sealed = self.cipher.seal(payload)?;
        self.inner.put(key, sealed);
        Ok(())
    }

    /// Opens the payload for a key and marks it most recently used.
    ///
    /// `Ok(None)` is a miss. `Err` is a hit whose sealed bytes would not open;
    /// the entry is dropped, so the caller recomputes rather than reading the
    /// same corrupt bytes again.
    pub fn load(&mut self, key: &UnderstandingKey) -> Result<Option<Vec<u8>>, CipherError> {
        let Some(sealed) = self.inner.get(key) else {
            return Ok(None);
        };
        let opened = self.cipher.open(sealed);
        if opened.is_err() {
            // A payload that will not open can never be read, so keeping it
            // only costs capacity. Drop it and let the caller recompute.
            self.inner.remove(key);
        }
        opened.map(Some)
    }

    /// The sealed bytes for a key, for writing to disk.
    ///
    /// This is ciphertext, never plaintext, and it is not marked as used: a
    /// backup or a flush must not change the eviction order.
    pub fn sealed(&self, key: &UnderstandingKey) -> Option<&[u8]> {
        self.inner.peek(key).map(Vec::as_slice)
    }

    /// Drops every entry for a content hash. Returns how many were dropped.
    pub fn invalidate_content(&mut self, content: ContentHash) -> usize {
        self.inner.retain(|key| key.content != content)
    }

    /// Drops every entry for a model version. Returns how many were dropped.
    ///
    /// A model update invalidates every result the old version produced.
    pub fn invalidate_model(&mut self, model: &ModelVersion) -> usize {
        self.inner.retain(|key| &key.model != model)
    }

    /// Drops every entry for a profile. Returns how many were dropped.
    pub fn invalidate_profile(&mut self, profile: &ProfileId) -> usize {
        self.inner.retain(|key| &key.profile != profile)
    }

    /// Drops every entry.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// The keys, most recently used first.
    pub fn keys(&self) -> impl Iterator<Item = &UnderstandingKey> {
        self.inner.keys()
    }
}

#[cfg(test)]
mod tests;
