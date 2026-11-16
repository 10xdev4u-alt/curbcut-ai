//! Tests for the opt-in telemetry exporter.

use super::*;
use crate::metric::MetricKey;

#[test]
fn an_exporter_starts_disabled() {
    let exporter = TelemetryExporter::new();
    assert!(!exporter.enabled());
    assert!(exporter.snapshot().is_empty());
    assert_eq!(exporter.export(), None);
    assert_eq!(exporter.export_result(), Err(TelemetryError::NotEnabled));
}

#[test]
fn export_is_none_while_disabled() {
    let mut exporter = TelemetryExporter::new();
    exporter.record(MetricKey::Errors, 3);
    assert_eq!(exporter.snapshot().get(MetricKey::Errors), 3);
    assert_eq!(
        exporter.export(),
        None,
        "counts are collected but not exported"
    );
}

#[test]
fn opt_in_enables_export() {
    let mut exporter = TelemetryExporter::new();
    exporter.record(MetricKey::Errors, 3);
    exporter.opt_in();

    assert!(exporter.enabled());
    let text = exporter.export().expect("an export");
    assert_eq!(text, "schema_version=1\nerrors=3\n");
    assert_eq!(exporter.export_result(), Ok(text));
}

#[test]
fn opt_out_disables_and_clears() {
    let mut exporter = TelemetryExporter::new();
    exporter.opt_in();
    exporter.record(MetricKey::Errors, 3);
    assert!(exporter.export().is_some());

    exporter.opt_out();
    assert!(!exporter.enabled());
    assert!(exporter.snapshot().is_empty());
    assert_eq!(exporter.export(), None);
}

#[test]
fn set_enabled_keeps_the_counts() {
    let mut exporter = TelemetryExporter::new();
    exporter.record(MetricKey::Errors, 3);

    exporter.set_enabled(true);
    assert!(exporter.enabled());
    assert_eq!(exporter.snapshot().get(MetricKey::Errors), 3);

    exporter.set_enabled(false);
    assert!(!exporter.enabled());
    assert_eq!(
        exporter.snapshot().get(MetricKey::Errors),
        3,
        "the counts stay"
    );
    assert_eq!(exporter.export(), None);
}

#[test]
fn the_exporter_records_through_the_snapshot() {
    let mut exporter = TelemetryExporter::new();
    exporter.increment(MetricKey::FramesCaptured, 4);
    exporter.bump(MetricKey::Errors);
    exporter.bump(MetricKey::Errors);

    assert_eq!(exporter.snapshot().get(MetricKey::FramesCaptured), 4);
    assert_eq!(exporter.snapshot().get(MetricKey::Errors), 2);

    exporter.clear();
    assert!(exporter.snapshot().is_empty());
    assert!(!exporter.enabled(), "clear leaves the flag alone");
}

#[test]
fn record_named_rejects_unknown_keys() {
    let mut exporter = TelemetryExporter::new();
    assert_eq!(exporter.record_named("errors", 1), Ok(()));
    assert_eq!(
        exporter.record_named("screen_text", 1),
        Err(TelemetryError::UnknownKey)
    );
    assert_eq!(exporter.snapshot().len(), 1);
}

#[test]
fn take_snapshot_leaves_an_empty_one() {
    let mut exporter = TelemetryExporter::new();
    exporter.record(MetricKey::Errors, 3);
    let taken = exporter.take_snapshot();

    assert_eq!(taken.get(MetricKey::Errors), 3);
    assert!(exporter.snapshot().is_empty());
    assert!(!exporter.enabled());
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
