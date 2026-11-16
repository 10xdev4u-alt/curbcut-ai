//! Tests for the counts-only snapshot.

use super::*;
use crate::metric::MetricKey;

#[test]
fn an_empty_snapshot_is_empty() {
    let snapshot = Snapshot::new();
    assert!(snapshot.is_empty());
    assert_eq!(snapshot.len(), 0);
    assert_eq!(snapshot.get(MetricKey::Errors), 0);
    assert!(!snapshot.contains(MetricKey::Errors));
    assert!(snapshot.keys().is_empty());
    assert_eq!(snapshot.export_lines(), 1, "only the schema marker");
}

#[test]
fn record_and_get() {
    let mut snapshot = Snapshot::new();
    snapshot.record(MetricKey::FramesCaptured, 10);
    snapshot.record(MetricKey::Errors, 2);

    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot.get(MetricKey::FramesCaptured), 10);
    assert_eq!(snapshot.get(MetricKey::Errors), 2);
    assert_eq!(
        snapshot.get(MetricKey::AnalysisRuns),
        0,
        "an unrecorded key is zero"
    );
    assert!(snapshot.contains(MetricKey::FramesCaptured));
    assert!(!snapshot.is_empty());

    snapshot.record(MetricKey::FramesCaptured, 11);
    assert_eq!(
        snapshot.get(MetricKey::FramesCaptured),
        11,
        "a record replaces"
    );
}

#[test]
fn record_named_rejects_unknown_keys() {
    let mut snapshot = Snapshot::new();
    assert_eq!(snapshot.record_named("errors", 3), Ok(()));
    assert_eq!(snapshot.get(MetricKey::Errors), 3);

    for name in ["screen_text", "", "session_id", "errors "] {
        assert_eq!(
            snapshot.record_named(name, 1),
            Err(TelemetryError::UnknownKey),
            "`{name}` was accepted"
        );
    }
    assert_eq!(snapshot.len(), 1, "a rejected key is not recorded");
}

#[test]
fn increment_and_bump_saturate() {
    let mut snapshot = Snapshot::new();
    snapshot.increment(MetricKey::FramesCaptured, 5);
    snapshot.increment(MetricKey::FramesCaptured, 5);
    assert_eq!(snapshot.get(MetricKey::FramesCaptured), 10);

    snapshot.bump(MetricKey::Errors);
    snapshot.bump(MetricKey::Errors);
    assert_eq!(snapshot.get(MetricKey::Errors), 2);

    snapshot.increment(MetricKey::FramesCaptured, u64::MAX);
    assert_eq!(snapshot.get(MetricKey::FramesCaptured), u64::MAX);
    snapshot.bump(MetricKey::FramesCaptured);
    assert_eq!(snapshot.get(MetricKey::FramesCaptured), u64::MAX);
}

#[test]
fn merge_takes_the_larger_value() {
    let mut left = Snapshot::new();
    left.record(MetricKey::Errors, 2);
    left.record(MetricKey::FramesCaptured, 7);

    let mut right = Snapshot::new();
    right.record(MetricKey::Errors, 5);
    right.record(MetricKey::AnalysisRuns, 1);

    left.merge(&right);
    assert_eq!(left.get(MetricKey::Errors), 5);
    assert_eq!(left.get(MetricKey::FramesCaptured), 7);
    assert_eq!(left.get(MetricKey::AnalysisRuns), 1);
    assert_eq!(left.len(), 3);
    assert_eq!(
        right.get(MetricKey::Errors),
        5,
        "the other side is unchanged"
    );
}

#[test]
fn sum_adds_both_sides() {
    let mut left = Snapshot::new();
    left.record(MetricKey::Errors, 2);
    let mut right = Snapshot::new();
    right.record(MetricKey::Errors, 3);
    right.record(MetricKey::FramesCaptured, 1);

    left.sum(&right);
    assert_eq!(left.get(MetricKey::Errors), 5);
    assert_eq!(left.get(MetricKey::FramesCaptured), 1);
}

#[test]
fn serialize_is_keys_and_numbers_only() {
    let mut snapshot = Snapshot::new();
    snapshot.record(MetricKey::FramesCaptured, 10);
    snapshot.record(MetricKey::Errors, 2);

    assert_eq!(
        snapshot.serialize(),
        "schema_version=1\nframes_captured=10\nerrors=2\n"
    );
}

#[test]
fn serialize_carries_no_content_or_identifiers() {
    let mut snapshot = Snapshot::new();
    snapshot.record(MetricKey::SessionsStarted, 4);
    snapshot.record(MetricKey::SpeechUtterances, 9);

    let text = snapshot.serialize();
    let debug = format!("{snapshot:?}");
    for line in text.lines().chain(debug.lines()) {
        assert!(!line.contains('"'), "a quote appeared: {line}");
        assert!(!line.contains(';'), "a separator appeared: {line}");
        assert!(
            !line.contains("session_id"),
            "an identifier appeared: {line}"
        );
        assert!(
            !line.contains("session-id"),
            "an identifier appeared: {line}"
        );
    }
    assert_eq!(snapshot.export_lines(), 3);
}

#[test]
fn keys_are_in_stable_order() {
    let mut snapshot = Snapshot::new();
    snapshot.record(MetricKey::PanicKeyPresses, 1);
    snapshot.record(MetricKey::Errors, 1);
    snapshot.record(MetricKey::FramesCaptured, 1);

    assert_eq!(
        snapshot.keys(),
        vec![
            MetricKey::FramesCaptured,
            MetricKey::Errors,
            MetricKey::PanicKeyPresses,
        ]
    );
    let text = snapshot.serialize();
    let frames = text.find("frames_captured").expect("frames");
    let errors = text.find("errors").expect("errors");
    let panic = text.find("panic_key_presses").expect("panic");
    assert!(frames < errors && errors < panic, "the order is not stable");
}

#[test]
fn parse_round_trips() {
    let mut snapshot = Snapshot::new();
    snapshot.record(MetricKey::FramesCaptured, 10);
    snapshot.record(MetricKey::Errors, 2);

    let parsed = Snapshot::parse(&snapshot.serialize()).expect("a parse");
    assert_eq!(parsed, snapshot);
    assert_eq!(parsed.get(MetricKey::FramesCaptured), 10);
    assert_eq!(parsed.get(MetricKey::Errors), 2);
}

#[test]
fn parse_ignores_blank_lines() {
    let text = "schema_version=1\n\nerrors=2\n\n";
    let parsed = Snapshot::parse(text).expect("a parse");
    assert_eq!(parsed.get(MetricKey::Errors), 2);
    assert_eq!(parsed.len(), 1);
}

#[test]
fn parse_rejects_unknown_keys() {
    let text = "schema_version=1\nscreen_text=1\n";
    assert_eq!(Snapshot::parse(text), Err(TelemetryError::UnknownKey));
}

#[test]
fn parse_rejects_malformed_lines() {
    assert_eq!(
        Snapshot::parse("schema_version=1\nerrors\n"),
        Err(TelemetryError::MalformedLine)
    );
    assert_eq!(
        Snapshot::parse("schema_version=1\nerrors=not-a-number\n"),
        Err(TelemetryError::MalformedLine)
    );
}

#[test]
fn parse_requires_the_schema_marker() {
    assert_eq!(
        Snapshot::parse("errors=2\n"),
        Err(TelemetryError::UnsupportedVersion)
    );
    assert_eq!(Snapshot::parse(""), Err(TelemetryError::UnsupportedVersion));
}

#[test]
fn parse_rejects_a_different_version() {
    assert_eq!(
        Snapshot::parse("schema_version=2\nerrors=2\n"),
        Err(TelemetryError::UnsupportedVersion)
    );
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/TELEMETRY.md");
    for word in ["telemetry", "counts", "allowlist", "opt-in", "export"] {
        assert!(
            doc.contains(word),
            "docs/TELEMETRY.md does not state `{word}`"
        );
    }
}
