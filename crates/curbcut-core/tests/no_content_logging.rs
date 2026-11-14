//! Integration tests for the redaction layer and the no-content-logging guard.
//!
//! The canary strings live in `tests/fixtures/content_canaries.txt`; this file
//! never repeats them, so a leak in a diagnostic is the only way one appears.

use std::sync::{Arc, Mutex};

use curbcut_core::redact::{RedactMode, Redactor, CANARY_PREFIX};

fn canary_path() -> String {
    format!(
        "{}/tests/fixtures/content_canaries.txt",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn canary_list() -> String {
    std::fs::read_to_string(canary_path()).expect("the canary fixture is present")
}

#[test]
fn the_fixture_and_the_loader_agree() {
    let from_file = Redactor::from_canary_file(&canary_path()).expect("fixture is present");
    let from_loader = Redactor::canaries();
    assert!(
        !from_file.secrets().is_empty(),
        "the fixture must have canaries"
    );
    assert_eq!(from_file.secrets(), from_loader.secrets());
    assert!(
        from_file
            .secrets()
            .iter()
            .all(|secret| secret.starts_with(CANARY_PREFIX)),
        "every canary must carry the prefix"
    );
}

#[test]
fn a_missing_fixture_is_an_error_not_an_empty_list() {
    let error = Redactor::from_canary_file("/nonexistent/canaries.txt")
        .expect_err("a missing fixture must not look like an empty list");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn every_fixture_canary_is_redacted() {
    let redactor = Redactor::canaries();
    for secret in redactor.secrets() {
        let message = format!("before {secret} after");
        let redacted = redactor.redact(&message);
        assert!(!redacted.contains(secret), "leaked: {redacted}");
        assert!(redacted.contains("[redacted]"), "{redacted}");
        assert!(redacted.starts_with("before "), "{redacted}");
        assert!(redacted.ends_with(" after"), "{redacted}");
    }
}

#[test]
fn a_canary_bearing_string_is_detected() {
    let redactor = Redactor::canaries();
    let secret = redactor.secrets()[0].clone();
    assert!(redactor.contains_any(&format!("x {secret} y")));
    assert!(!redactor.contains_any("nothing secret here"));
}

#[test]
fn hash_mode_keeps_a_stable_placeholder() {
    let redactor = Redactor::canaries().with_mode(RedactMode::Hash);
    let secret = redactor.secrets()[0].clone();
    let first = redactor.redact(&secret);
    let second = redactor.redact(&secret);
    assert_eq!(first, second);
    assert!(!first.contains(&secret));
    assert!(first.starts_with('[') && first.ends_with(']'), "{first}");
}

#[test]
fn the_logger_never_passes_a_canary_to_its_sink() {
    let sink = Arc::new(Mutex::new(String::new()));
    let captured = sink.clone();
    let logger = curbcut_core::redact::Logger::with_sink(
        Redactor::canaries(),
        Box::new(move |line| {
            let mut buffer = captured.lock().expect("sink");
            buffer.push_str(line);
            buffer.push('\n');
        }),
    );
    for secret in logger.redactor().secrets() {
        logger.info(&format!("element label={secret} count=1"));
    }
    let output = sink.lock().expect("sink").clone();
    for secret in Redactor::canaries().secrets() {
        assert!(!output.contains(secret), "leaked: {output}");
    }
    assert!(output.contains("[redacted]"), "{output}");
}

#[test]
fn an_empty_redactor_is_a_no_op() {
    let redactor = Redactor::new();
    assert!(redactor.secrets().is_empty());
    assert_eq!(redactor.redact("unchanged"), "unchanged");
    assert!(!redactor.contains_any("unchanged"));
}

#[test]
fn the_fixture_is_parseable() {
    let list = canary_list();
    let redactor = Redactor::from_canary_list(&list);
    assert!(!redactor.secrets().is_empty());
    assert_eq!(
        redactor.secrets().len(),
        list.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .count()
    );
}
