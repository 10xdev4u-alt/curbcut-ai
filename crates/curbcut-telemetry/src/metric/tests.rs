//! Tests for the allowlisted metric keys.

use super::*;

#[test]
fn the_allowlist_is_stable_and_unique() {
    assert_eq!(MetricKey::ALL.len(), 16);
    let mut names: Vec<&str> = MetricKey::ALL.iter().map(|key| key.as_str()).collect();
    assert!(names.iter().all(|name| !name.is_empty()));
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count, "two keys share a name");
}

#[test]
fn every_name_resolves_back_to_its_key() {
    for key in MetricKey::ALL {
        assert_eq!(MetricKey::from_name(key.as_str()), Some(key));
    }
}

#[test]
fn names_outside_the_allowlist_are_rejected() {
    for name in [
        "",
        "screen_text",
        "window_title",
        "session_id",
        "Frames_Captured",
        "frames captured",
        "frames-captured",
        "frames_captured ",
    ] {
        assert_eq!(MetricKey::from_name(name), None, "`{name}` was accepted");
    }
}

#[test]
fn units_are_counts_except_uptime() {
    assert_eq!(MetricKey::UptimeSeconds.unit(), "seconds");
    for key in MetricKey::ALL {
        if key != MetricKey::UptimeSeconds {
            assert_eq!(key.unit(), "count", "{} has the wrong unit", key.as_str());
        }
    }
}

#[test]
fn a_metric_holds_a_key_and_a_count() {
    let metric = Metric::new(MetricKey::Errors, 7);
    assert_eq!(metric.key(), MetricKey::Errors);
    assert_eq!(metric.value(), 7);
    assert_eq!(metric.unit(), "count");
    assert_eq!(Metric::one(MetricKey::Errors).value(), 1);
    assert_eq!(Metric::one(MetricKey::Errors).unit(), "count");
}

#[test]
fn increment_saturates_at_the_maximum() {
    let mut metric = Metric::new(MetricKey::FramesCaptured, u64::MAX - 1);
    metric.increment(1);
    assert_eq!(metric.value(), u64::MAX);
    metric.increment(1);
    assert_eq!(metric.value(), u64::MAX, "the count saturates");
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
