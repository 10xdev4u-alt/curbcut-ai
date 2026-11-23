//! Property tests for the core crate.
//!
//! Invariants, not examples: each case generates fresh input and checks a
//! property that must hold for all of it. A failure reports the seed and the
//! case index so the case can be replayed.

use curbcut_core::config::{ConfigSlot, ReadingLevel};
use curbcut_core::hash::{hmac_sha256, sha256};
use curbcut_core::id::{ChunkId, ElementId, ProfileId, WindowId};
use curbcut_core::prop::{self, Rng};
use curbcut_core::redact::{RedactMode, Redactor};

/// A profile slug: letters, digits, `-`, and `_` only, which is exactly what
/// the config parser accepts. The generic alphabet also contains `"`, `\n`, and
/// `\t`, which would make the TOML key invalid for a reason unrelated to the
/// property under test.
fn slug(rng: &mut Rng) -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789-_";
    let len = rng.between(1, 12) as usize;
    (0..len)
        .map(|_| ALPHABET[rng.below(ALPHABET.len() as u64) as usize] as char)
        .collect()
}

#[test]
fn a_rejected_document_never_replaces_the_active_config() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x2000, cases, |rng, _| {
        let slot = ConfigSlot::default();
        let before = slot.get();
        let revision = slot.revision();
        let len = rng.between(0, 400) as usize;
        let document = rng.ascii(len);

        match slot.apply_toml(&document) {
            Ok(next) => {
                if slot.get() != next {
                    return Err("the applied config is not the current one".to_string());
                }
                if slot.revision() != revision + 1 {
                    return Err("a successful apply did not bump the revision".to_string());
                }
                next.validate()
                    .map_err(|_| "the applied config is invalid")?;
            }
            Err(_) => {
                if slot.get() != before {
                    return Err("a rejected document replaced the config".to_string());
                }
                if slot.revision() != revision {
                    return Err("a rejected document bumped the revision".to_string());
                }
            }
        }
        Ok(())
    })
    .expect("a rejected document never replaces the active config");
}

#[test]
fn a_valid_profile_document_round_trips_through_the_slot() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x2001, cases, |rng, _| {
        let slug = slug(rng);
        let hot_reload = rng.bool();
        let stop_motion = rng.bool();
        let reduce_transparency = rng.bool();
        let short_paths = rng.bool();
        let reading_level = if rng.bool() {
            ReadingLevel::Keep
        } else {
            ReadingLevel::Grade(rng.between(1, 12) as u8)
        };
        let interruptions = rng
            .choose(&["inbox", "block", "allow"])
            .expect("the mode table is non-empty");
        let document = format!(
            "hot_reload = {hot_reload}\n\
             [profile.{slug}]\n\
             stop_motion = {stop_motion}\n\
             reduce_transparency = {reduce_transparency}\n\
             short_paths = {short_paths}\n\
             reading_level = \"{}\"\n\
             interruptions = \"{interruptions}\"\n",
            reading_level.as_str()
        );

        let expected_id = match slug.as_str() {
            "focus" => ProfileId::focus(),
            "read" => ProfileId::read(),
            "simple" => ProfileId::simple(),
            "calm" => ProfileId::calm(),
            _ => ProfileId::custom(&slug),
        };
        let slot = ConfigSlot::default();
        let applied = slot
            .apply_toml(&document)
            .map_err(|_| "a well-formed document was rejected")?;
        applied
            .validate()
            .map_err(|_| "the applied config is invalid")?;
        if applied.hot_reload != hot_reload {
            return Err("the hot_reload flag did not round-trip".to_string());
        }
        let settings = applied
            .profiles
            .get(&expected_id)
            .ok_or("the profile is missing after applying")?;
        if settings.stop_motion != stop_motion
            || settings.reduce_transparency != reduce_transparency
            || settings.short_paths != short_paths
            || settings.reading_level != reading_level
        {
            return Err("a profile setting did not round-trip".to_string());
        }
        let revision = slot.revision();
        let again = slot
            .apply_toml(&document)
            .map_err(|_| "the same document was rejected twice")?;
        if again != applied {
            return Err("re-applying the same document changed the config".to_string());
        }
        if slot.revision() != revision + 1 {
            return Err("a re-apply did not bump the revision".to_string());
        }
        Ok(())
    })
    .expect("a valid profile document round-trips through the slot");
}

#[test]
fn a_boolean_setting_round_trips_and_reapplying_is_idempotent() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x2008, cases, |rng, _| {
        let hot_reload = rng.bool();
        let document = format!("hot_reload = {hot_reload}\n");
        let slot = ConfigSlot::default();
        let applied = slot
            .apply_toml(&document)
            .map_err(|_| "the document was rejected")?;
        if applied.hot_reload != hot_reload {
            return Err("the boolean did not round-trip".to_string());
        }
        let revision = slot.revision();
        let again = slot
            .apply_toml(&document)
            .map_err(|_| "the same document was rejected twice")?;
        if again != applied {
            return Err("re-applying the same document changed the config".to_string());
        }
        if slot.revision() != revision + 1 {
            return Err("a re-apply did not bump the revision".to_string());
        }
        Ok(())
    })
    .expect("a boolean setting round-trips and re-applying is idempotent");
}

#[test]
fn redaction_is_idempotent_and_removes_every_canary() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x2002, cases, |rng, _| {
        let redactor = Redactor::canaries();
        let secrets = redactor.secrets();
        if secrets.is_empty() {
            return Err("the canary fixture is empty".to_string());
        }
        let len = rng.between(0, 200) as usize;
        let mut text = rng.ascii(len);
        for _ in 0..rng.between(1, 6) as usize {
            let secret = rng.choose(secrets).expect("non-empty");
            let at = rng.between(0, text.len() as u64) as usize;
            text.insert_str(at, secret);
        }

        let once = redactor.redact(&text);
        for secret in secrets {
            if once.contains(secret) {
                return Err("a canary survived redaction".to_string());
            }
        }
        let twice = redactor.redact(&once);
        if once != twice {
            return Err("redacting twice changed the text".to_string());
        }
        Ok(())
    })
    .expect("redaction is idempotent and removes every canary");
}

#[test]
fn hash_mode_replaces_a_canary_with_a_stable_placeholder() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x2003, cases, |rng, _| {
        let redactor = Redactor::new().with_mode(RedactMode::Hash);
        let len = rng.between(0, 200) as usize;
        let text = rng.ascii(len);
        let once = redactor.redact(&text);
        let twice = redactor.redact(&once);
        if once != twice {
            return Err("hash redaction is not stable".to_string());
        }
        Ok(())
    })
    .expect("hash mode is stable");
}

#[test]
fn ids_preserve_numeric_order_and_value() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x2004, cases, |rng, _| {
        let mut ids: Vec<u64> = (0..rng.between(2, 32))
            .map(|_| rng.between(0, u64::MAX))
            .collect();
        let mut elements: Vec<ElementId> = ids.iter().map(|value| ElementId(*value)).collect();
        elements.sort_unstable();
        ids.sort_unstable();
        for (element, value) in elements.iter().zip(&ids) {
            if element.get() != *value {
                return Err("the id lost its numeric value".to_string());
            }
        }
        let windows: Vec<WindowId> = ids.iter().map(|value| WindowId(*value)).collect();
        let chunks: Vec<ChunkId> = ids.iter().map(|value| ChunkId(*value)).collect();
        if windows.len() != ids.len() || chunks.len() != ids.len() {
            return Err("an id vector changed length".to_string());
        }
        Ok(())
    })
    .expect("ids preserve numeric order and value");
}

#[test]
fn a_digest_round_trips_through_its_hex_form() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x2005, cases, |rng, _| {
        let len = rng.between(0, 512) as usize;
        let message = rng.bytes(len);
        let digest = sha256(&message);
        if !digest.matches_hex(&digest.as_hex()) {
            return Err("the digest does not match its own hex form".to_string());
        }
        let parsed = curbcut_core::hash::Digest::from_hex(&digest.as_hex())
            .ok_or("the hex form did not parse")?;
        if parsed.as_bytes() != digest.as_bytes() {
            return Err("the parsed digest differs".to_string());
        }
        let again = sha256(&message);
        if again.as_bytes() != digest.as_bytes() {
            return Err("the same message produced two digests".to_string());
        }
        Ok(())
    })
    .expect("a digest round-trips through its hex form");
}

#[test]
fn a_keyed_digest_depends_on_the_key_and_the_message() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x2006, cases, |rng, _| {
        let message_len = rng.between(0, 128) as usize;
        let message = rng.bytes(message_len);
        // A key at least one SHA-256 block long. HMAC zero-pads a short key, so
        // an empty key and a one-byte `0x00` key are the same HMAC key; a key
        // that fills the block keeps the "different key" invariant meaningful.
        let key_len = rng.between(64, 192) as usize;
        let key = rng.bytes(key_len);
        let digest = hmac_sha256(&key, &message);
        if hmac_sha256(&key, &message).as_bytes() != digest.as_bytes() {
            return Err("the same key and message produced two digests".to_string());
        }
        let other_message = {
            let mut mutated = message.clone();
            if mutated.is_empty() {
                mutated.push(0);
            } else {
                mutated[0] ^= 0x01;
            }
            mutated
        };
        if message != other_message
            && hmac_sha256(&key, &other_message).as_bytes() == digest.as_bytes()
        {
            return Err("a different message produced the same digest".to_string());
        }
        let other_key = {
            let mut mutated = key.clone();
            if mutated.is_empty() {
                mutated.push(0);
            } else {
                mutated[0] ^= 0x01;
            }
            mutated
        };
        if key != other_key && hmac_sha256(&other_key, &message).as_bytes() == digest.as_bytes() {
            return Err("a different key produced the same digest".to_string());
        }
        Ok(())
    })
    .expect("a keyed digest depends on the key and the message");
}

#[test]
fn reading_levels_round_trip_through_their_string_form() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x2007, cases, |rng, _| {
        let level = if rng.bool() {
            ReadingLevel::Keep
        } else {
            ReadingLevel::Grade(rng.between(1, 12) as u8)
        };
        let text = level.as_str();
        let parsed = ReadingLevel::parse(&text).map_err(|_| "the string form did not parse")?;
        if parsed != level {
            return Err("the parsed level differs".to_string());
        }
        Ok(())
    })
    .expect("reading levels round-trip through their string form");
}
