//! Tests for the sealed understanding-result cache.

use std::cell::Cell;
use std::rc::Rc;

use super::*;

/// A deterministic, reversible probe cipher.
///
/// It is a test double, not a security control: it proves the cache stores the
/// cipher's output and never the payload.
#[derive(Debug, Clone)]
struct ProbeCipher {
    key: u8,
}

impl Cipher for ProbeCipher {
    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, CipherError> {
        let mut sealed = Vec::with_capacity(plaintext.len() + 1);
        sealed.push(self.key);
        sealed.extend(plaintext.iter().map(|byte| byte ^ self.key));
        Ok(sealed)
    }

    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, CipherError> {
        let (first, rest) = sealed.split_first().ok_or(CipherError::Corrupt)?;
        if *first != self.key {
            return Err(CipherError::Corrupt);
        }
        Ok(rest.iter().map(|byte| byte ^ self.key).collect())
    }
}

/// A cipher that seals but never opens, standing in for a lost key.
#[derive(Debug, Clone)]
struct NeverOpens;

impl Cipher for NeverOpens {
    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, CipherError> {
        Ok(plaintext.to_vec())
    }

    fn open(&self, _: &[u8]) -> Result<Vec<u8>, CipherError> {
        Err(CipherError::Corrupt)
    }
}

/// A cipher whose key is present at first and then goes away.
#[derive(Debug, Clone)]
struct FlakyKey {
    available: Rc<Cell<bool>>,
}

impl Cipher for FlakyKey {
    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, CipherError> {
        if !self.available.get() {
            return Err(CipherError::NoKey);
        }
        Ok(plaintext.to_vec())
    }

    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, CipherError> {
        if !self.available.get() {
            return Err(CipherError::NoKey);
        }
        Ok(sealed.to_vec())
    }
}

/// A cipher with no key at all.
#[derive(Debug, Clone)]
struct NoKey;

impl Cipher for NoKey {
    fn seal(&self, _: &[u8]) -> Result<Vec<u8>, CipherError> {
        Err(CipherError::NoKey)
    }

    fn open(&self, _: &[u8]) -> Result<Vec<u8>, CipherError> {
        Err(CipherError::NoKey)
    }
}

fn key(content: u64, model: &str, profile: &str) -> UnderstandingKey {
    UnderstandingKey::new(
        ContentHash::new(content),
        ModelVersion::new(model),
        ProfileId::from(profile),
    )
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[test]
fn the_documented_rules_match_the_docs() {
    let doc = include_str!("../../../../docs/UNDERSTANDING_CACHE.md");
    for word in [
        "content hash",
        "model version",
        "profile",
        "LRU",
        "plaintext",
        "Cipher",
    ] {
        assert!(
            doc.contains(word),
            "docs/UNDERSTANDING_CACHE.md does not state `{word}`"
        );
    }
}

#[test]
fn a_stored_payload_round_trips() {
    let mut cache = UnderstandingCache::new(4, ProbeCipher { key: 7 });
    let payload = b"simplified: three short sentences";

    cache.store(key(1, "m1", "read"), payload).unwrap();
    assert_eq!(cache.len(), 1);
    assert_eq!(
        cache.load(&key(1, "m1", "read")).unwrap().as_deref(),
        Some(&payload[..])
    );
}

#[test]
fn a_missing_key_is_a_miss_not_an_error() {
    let mut cache = UnderstandingCache::new(4, ProbeCipher { key: 7 });
    assert_eq!(cache.load(&key(1, "m1", "read")).unwrap(), None);
    assert!(cache.is_empty());
}

#[test]
fn the_content_hash_is_part_of_the_key() {
    let mut cache = UnderstandingCache::new(4, ProbeCipher { key: 7 });
    cache.store(key(1, "m1", "read"), b"one").unwrap();

    assert_eq!(cache.load(&key(2, "m1", "read")).unwrap(), None);
    assert_eq!(
        cache.load(&key(1, "m1", "read")).unwrap().as_deref(),
        Some(&b"one"[..])
    );
}

#[test]
fn the_model_version_is_part_of_the_key() {
    let mut cache = UnderstandingCache::new(4, ProbeCipher { key: 7 });
    cache.store(key(1, "m1", "read"), b"old").unwrap();

    assert_eq!(cache.load(&key(1, "m2", "read")).unwrap(), None);
    assert_eq!(
        cache.load(&key(1, "m1", "read")).unwrap().as_deref(),
        Some(&b"old"[..])
    );
}

#[test]
fn the_profile_is_part_of_the_key() {
    let mut cache = UnderstandingCache::new(4, ProbeCipher { key: 7 });
    cache.store(key(1, "m1", "read"), b"read").unwrap();

    assert_eq!(cache.load(&key(1, "m1", "focus")).unwrap(), None);
    assert_eq!(
        cache.load(&key(1, "m1", "read")).unwrap().as_deref(),
        Some(&b"read"[..])
    );
}

#[test]
fn a_hit_is_deterministic_for_the_same_key() {
    let mut cache = UnderstandingCache::new(4, ProbeCipher { key: 7 });
    let payload = b"same answer every time";
    cache.store(key(9, "m1", "calm"), payload).unwrap();

    for _ in 0..5 {
        assert_eq!(
            cache.load(&key(9, "m1", "calm")).unwrap().as_deref(),
            Some(&payload[..])
        );
    }
}

#[test]
fn the_stored_bytes_are_the_cipher_output_and_contain_no_plaintext() {
    let cipher = ProbeCipher { key: 7 };
    let mut cache = UnderstandingCache::new(4, cipher.clone());
    let marker = b"SCREEN-TEXT-9F3A";
    let payload = b"derived from SCREEN-TEXT-9F3A";

    cache.store(key(1, "m1", "read"), payload).unwrap();

    let sealed = cache.sealed(&key(1, "m1", "read")).unwrap();
    assert_eq!(sealed, cipher.seal(payload).unwrap().as_slice());
    assert_ne!(sealed, &payload[..]);
    assert!(
        !contains(sealed, marker),
        "the stored bytes must not contain the payload's text"
    );
    assert!(
        !contains(sealed, b"derived"),
        "the stored bytes must not contain a plaintext fragment"
    );
}

#[test]
fn the_sealed_bytes_do_not_open_with_another_key() {
    let sealed = ProbeCipher { key: 7 }.seal(b"payload").unwrap();
    assert_eq!(
        ProbeCipher { key: 8 }.open(&sealed),
        Err(CipherError::Corrupt)
    );
}

#[test]
fn sealed_does_not_change_the_recency_order() {
    let mut cache = UnderstandingCache::new(2, ProbeCipher { key: 7 });
    cache.store(key(1, "m1", "read"), b"one").unwrap();
    cache.store(key(2, "m1", "read"), b"two").unwrap();

    assert!(cache.sealed(&key(1, "m1", "read")).is_some());
    cache.store(key(3, "m1", "read"), b"three").unwrap();

    assert!(
        cache.sealed(&key(1, "m1", "read")).is_none(),
        "reading the sealed bytes must not look like a use"
    );
    assert!(cache.sealed(&key(2, "m1", "read")).is_some());
}

#[test]
fn a_payload_that_will_not_open_is_dropped() {
    let mut cache = UnderstandingCache::new(4, NeverOpens);
    cache.store(key(1, "m1", "read"), b"payload").unwrap();
    assert_eq!(cache.len(), 1);

    assert_eq!(cache.load(&key(1, "m1", "read")), Err(CipherError::Corrupt));
    assert!(
        cache.is_empty(),
        "a payload that will not open is dropped, not kept"
    );
}

#[test]
fn a_missing_key_does_not_drop_a_valid_entry() {
    let available = Rc::new(Cell::new(true));
    let mut cache = UnderstandingCache::new(
        4,
        FlakyKey {
            available: available.clone(),
        },
    );
    cache.store(key(1, "m1", "read"), b"payload").unwrap();

    available.set(false);
    assert_eq!(cache.load(&key(1, "m1", "read")), Err(CipherError::NoKey));
    assert_eq!(
        cache.len(),
        1,
        "the sealed bytes are still readable once the key returns"
    );

    available.set(true);
    assert_eq!(
        cache.load(&key(1, "m1", "read")).unwrap().as_deref(),
        Some(&b"payload"[..])
    );
}

#[test]
fn a_cipher_with_no_key_stores_nothing() {
    let mut cache = UnderstandingCache::new(4, NoKey);
    assert_eq!(
        cache.store(key(1, "m1", "read"), b"payload"),
        Err(CipherError::NoKey)
    );
    assert!(cache.is_empty());
    assert_eq!(
        cache.load(&key(1, "m1", "read")).unwrap(),
        None,
        "nothing was stored, so the lookup is a miss"
    );
}

#[test]
fn a_zero_capacity_cache_stores_nothing() {
    let mut cache = UnderstandingCache::new(0, ProbeCipher { key: 7 });
    cache.store(key(1, "m1", "read"), b"payload").unwrap();

    assert!(cache.is_empty());
    assert_eq!(cache.capacity(), 0);
    assert_eq!(cache.load(&key(1, "m1", "read")).unwrap(), None);
}

#[test]
fn storing_an_existing_key_replaces_it_without_growing() {
    let mut cache = UnderstandingCache::new(4, ProbeCipher { key: 7 });
    cache.store(key(1, "m1", "read"), b"old").unwrap();
    cache.store(key(2, "m1", "read"), b"other").unwrap();
    cache.store(key(1, "m1", "read"), b"new").unwrap();

    assert_eq!(cache.len(), 2);
    assert_eq!(
        cache.load(&key(1, "m1", "read")).unwrap().as_deref(),
        Some(&b"new"[..])
    );
}

#[test]
fn eviction_is_least_recently_used_under_pressure() {
    let mut cache = UnderstandingCache::new(4, ProbeCipher { key: 7 });
    for content in 0..50 {
        cache.store(key(content, "m1", "read"), b"payload").unwrap();
        assert!(cache.len() <= 4);
    }

    assert_eq!(cache.len(), 4);
    let contents: Vec<u64> = cache
        .keys()
        .map(|candidate| candidate.content().get())
        .collect();
    assert_eq!(contents, vec![49, 48, 47, 46]);
}

#[test]
fn a_hit_protects_an_entry_from_eviction() {
    let mut cache = UnderstandingCache::new(2, ProbeCipher { key: 7 });
    cache.store(key(1, "m1", "read"), b"one").unwrap();
    cache.store(key(2, "m1", "read"), b"two").unwrap();

    assert_eq!(
        cache.load(&key(1, "m1", "read")).unwrap().as_deref(),
        Some(&b"one"[..])
    );
    cache.store(key(3, "m1", "read"), b"three").unwrap();

    assert!(cache.sealed(&key(1, "m1", "read")).is_some());
    assert!(cache.sealed(&key(2, "m1", "read")).is_none());
}

#[test]
fn invalidate_content_drops_every_profile_and_model() {
    let mut cache = UnderstandingCache::new(8, ProbeCipher { key: 7 });
    cache.store(key(1, "m1", "read"), b"a").unwrap();
    cache.store(key(1, "m1", "focus"), b"b").unwrap();
    cache.store(key(1, "m2", "read"), b"c").unwrap();
    cache.store(key(2, "m1", "read"), b"d").unwrap();

    assert_eq!(cache.invalidate_content(ContentHash::new(1)), 3);
    assert_eq!(cache.len(), 1);
    assert!(cache.sealed(&key(2, "m1", "read")).is_some());
}

#[test]
fn invalidate_model_drops_every_result_from_that_version() {
    let mut cache = UnderstandingCache::new(8, ProbeCipher { key: 7 });
    cache.store(key(1, "m1", "read"), b"a").unwrap();
    cache.store(key(2, "m1", "read"), b"b").unwrap();
    cache.store(key(1, "m2", "read"), b"c").unwrap();

    assert_eq!(cache.invalidate_model(&ModelVersion::new("m1")), 2);
    assert_eq!(cache.len(), 1);
    assert!(cache.sealed(&key(1, "m2", "read")).is_some());
}

#[test]
fn invalidate_profile_drops_every_result_for_that_profile() {
    let mut cache = UnderstandingCache::new(8, ProbeCipher { key: 7 });
    cache.store(key(1, "m1", "read"), b"a").unwrap();
    cache.store(key(2, "m1", "read"), b"b").unwrap();
    cache.store(key(1, "m1", "focus"), b"c").unwrap();

    assert_eq!(cache.invalidate_profile(&ProfileId::from("read")), 2);
    assert_eq!(cache.len(), 1);
    assert!(cache.sealed(&key(1, "m1", "focus")).is_some());
}

#[test]
fn evicting_an_entry_zeroizes_the_sealed_bytes() {
    curbcut_core::secure::reset_zeroized_on_drop();
    let mut cache = UnderstandingCache::new(2, ProbeCipher { key: 7 });
    cache.store(key(1, "m1", "read"), b"one").unwrap();
    cache.store(key(2, "m1", "read"), b"two").unwrap();
    assert_eq!(curbcut_core::secure::zeroized_on_drop(), 0);

    cache.store(key(3, "m1", "read"), b"three").unwrap();
    assert_eq!(
        curbcut_core::secure::zeroized_on_drop(),
        1,
        "the evicted entry must be overwritten, not merely unlinked"
    );
}

#[test]
fn clear_zeroizes_every_sealed_entry() {
    curbcut_core::secure::reset_zeroized_on_drop();
    let mut cache = UnderstandingCache::new(4, ProbeCipher { key: 7 });
    cache.store(key(1, "m1", "read"), b"a").unwrap();
    cache.store(key(2, "m1", "read"), b"b").unwrap();
    assert_eq!(curbcut_core::secure::zeroized_on_drop(), 0);

    cache.clear();
    assert_eq!(curbcut_core::secure::zeroized_on_drop(), 2);
}

#[test]
fn clear_drops_every_sealed_entry() {
    let mut cache = UnderstandingCache::new(4, ProbeCipher { key: 7 });
    cache.store(key(1, "m1", "read"), b"a").unwrap();
    cache.clear();

    assert!(cache.is_empty());
    assert!(cache.sealed(&key(1, "m1", "read")).is_none());
}

#[test]
fn a_model_version_round_trips_through_its_string_form() {
    let version = ModelVersion::from("1.4.2");
    assert_eq!(version.as_str(), "1.4.2");
    assert_eq!(ModelVersion::new("1.4.2"), version);
}

#[test]
fn a_key_exposes_its_parts() {
    let version = ModelVersion::new("m1");
    let profile = ProfileId::focus();
    let key = UnderstandingKey::new(ContentHash::new(42), version.clone(), profile.clone());

    assert_eq!(key.content().get(), 42);
    assert_eq!(key.model(), &version);
    assert_eq!(key.profile(), &profile);
}

#[test]
fn a_cipher_error_names_no_detail() {
    for error in [
        CipherError::NoKey,
        CipherError::SealFailed,
        CipherError::Corrupt,
    ] {
        let text = error.to_string();
        assert!(!text.is_empty());
        assert!(!text.contains("payload"));
        assert!(!text.contains("keychain"));
    }
}
