//! Fuzz tests for the core parsers, under a wall-clock budget.
//!
//! Every case is generated from a seed, so a crash reports the seed and the
//! case index and can be replayed exactly. CI raises the case count with
//! `CURBCUT_PROP_CASES` and caps the run with `CURBCUT_FUZZ_BUDGET_MS`; running
//! out of budget stops the run rather than failing it.
//!
//! The alphabet is structural on purpose: a parser that only ever sees letters
//! will not crash on a bracket. Generated strings are lowercase, so they cannot
//! spell a canary prefix; the canary tests build their canaries from the
//! redactor's own list.

use std::time::Duration;

use curbcut_core::config::Config;
use curbcut_core::hash::{sha256, Digest};
use curbcut_core::prop::{self, Rng};
use curbcut_core::redact::Redactor;

/// Letters, digits, whitespace, and the structural characters a document uses.
const FUZZ_ALPHABET: &[u8] = b"abc0123 \t\n\r\"'[]{}#=.,-+_\\:/()`~!@$%^&*|<>?";

/// Hex digits plus a few lookalikes, for the hex parser.
const HEX_ALPHABET: &[u8] = b"0123456789abcdefABCDEFghz";

fn fuzz_text(rng: &mut Rng) -> String {
    let len = rng.between(0, 256) as usize;
    (0..len)
        .map(|_| FUZZ_ALPHABET[rng.below(FUZZ_ALPHABET.len() as u64) as usize] as char)
        .collect()
}

fn hex_text(rng: &mut Rng) -> String {
    let len = rng.between(0, 80) as usize;
    (0..len)
        .map(|_| HEX_ALPHABET[rng.below(HEX_ALPHABET.len() as u64) as usize] as char)
        .collect()
}

fn budget() -> Duration {
    Duration::from_millis(prop::budget_ms(
        "CURBCUT_FUZZ_BUDGET_MS",
        prop::DEFAULT_BUDGET_MS,
    ))
}

#[test]
fn the_config_parser_never_panics() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run_budgeted(0x3000, cases, budget(), |rng, _| {
        let text = fuzz_text(rng);
        let _ = Config::from_toml(&text);
        Ok(())
    })
    .expect("the config parser never panics");
}

#[test]
fn an_accepted_config_always_validates() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run_budgeted(0x3001, cases, budget(), |rng, _| {
        let text = fuzz_text(rng);
        if let Ok(config) = Config::from_toml(&text) {
            config
                .validate()
                .map_err(|_| "an accepted document failed validation")?;
        }
        Ok(())
    })
    .expect("an accepted config always validates");
}

#[test]
fn a_fuzzed_canary_list_still_redacts_every_entry() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run_budgeted(0x3002, cases, budget(), |rng, _| {
        let mut list = String::new();
        for _ in 0..rng.between(0, 8) as usize {
            match rng.between(0, 3) {
                0 => {
                    let len = rng.between(0, 24) as usize;
                    list.push_str("# ");
                    list.push_str(&rng.ascii(len));
                    list.push('\n');
                }
                1 => list.push('\n'),
                _ => {
                    let len = rng.between(1, 40) as usize;
                    list.push_str(&rng.ascii(len));
                    list.push('\n');
                }
            }
        }
        let redactor = Redactor::from_canary_list(&list);

        let text_len = rng.between(0, 200) as usize;
        let mut text = rng.ascii(text_len);
        for secret in redactor.secrets() {
            if rng.bool() {
                let at = rng.between(0, text.len() as u64) as usize;
                text.insert_str(at, secret);
            }
        }
        let redacted = redactor.redact(&text);
        for secret in redactor.secrets() {
            if redacted.contains(secret.as_str()) {
                return Err("a registered canary survived redaction".to_string());
            }
        }
        Ok(())
    })
    .expect("a fuzzed canary list still redacts every entry");
}

#[test]
fn the_canary_fixture_redacts_itself_from_a_mixed_document() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run_budgeted(0x3003, cases, budget(), |rng, _| {
        let redactor = Redactor::canaries();
        let text_len = rng.between(0, 200) as usize;
        let mut text = rng.ascii(text_len);
        for _ in 0..rng.between(1, 6) as usize {
            let secret = rng.choose(redactor.secrets()).expect("non-empty");
            let at = rng.between(0, text.len() as u64) as usize;
            text.insert_str(at, secret);
        }
        let redacted = redactor.redact(&text);
        if redactor.contains_any(&redacted) {
            return Err("a fixture canary survived redaction".to_string());
        }
        Ok(())
    })
    .expect("the canary fixture redacts itself from a mixed document");
}

#[test]
fn hex_parsing_never_panics_and_round_trips() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run_budgeted(0x3004, cases, budget(), |rng, _| {
        let text = hex_text(rng);
        if let Some(digest) = Digest::from_hex(&text) {
            if !digest.matches_hex(&digest.as_hex()) {
                return Err("a parsed digest does not match its own hex form".to_string());
            }
            let reparsed = Digest::from_hex(&digest.as_hex())
                .ok_or("the canonical hex form did not reparse")?;
            if reparsed != digest {
                return Err("the reparsed digest differs".to_string());
            }
        }
        let len = rng.between(0, 128) as usize;
        let message = rng.bytes(len);
        let digest = sha256(&message);
        if !digest.matches_hex(&digest.as_hex()) {
            return Err("a computed digest does not match its own hex form".to_string());
        }
        Ok(())
    })
    .expect("hex parsing never panics and round-trips");
}

#[test]
fn arbitrary_bytes_never_panic_the_hash_layer() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run_budgeted(0x3005, cases, budget(), |rng, _| {
        let len = rng.between(0, 512) as usize;
        let bytes = rng.bytes(len);
        let _ = sha256(&bytes);
        let _ = Digest::from_hex(&String::from_utf8_lossy(&bytes));
        Ok(())
    })
    .expect("arbitrary bytes never panic the hash layer");
}
