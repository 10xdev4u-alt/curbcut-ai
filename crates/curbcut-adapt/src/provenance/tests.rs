//! Tests for the provenance panel.

use curbcut_core::id::RuleId;

use super::*;
use crate::confirm::ActionId;
use crate::settings::{SettingKey, SettingValue};

fn rule(slug: &str) -> RuleId {
    RuleId::new(slug)
}

fn action(slug: &str) -> ActionId {
    ActionId::new(slug)
}

fn record(
    panel: &mut ProvenancePanel,
    action_slug: &str,
    rule_slug: &str,
    setting: SettingKey,
    original: OriginalValue,
) -> ProvenanceId {
    panel.record(
        action(action_slug),
        rule(rule_slug),
        setting,
        original,
        1_000,
    )
}

#[test]
fn a_change_links_to_its_rule() {
    let mut panel = ProvenancePanel::new();
    let id = record(
        &mut panel,
        "dim-on",
        "focus-mode",
        SettingKey::DimEnabled,
        OriginalValue::setting(SettingValue::Bool(false)),
    );

    let entry = panel.get(id).expect("the entry exists");
    assert_eq!(entry.rule(), &rule("focus-mode"));
    assert_eq!(entry.action().as_str(), "dim-on");
    assert_eq!(entry.setting(), SettingKey::DimEnabled);
    assert_eq!(entry.changed_at_ms(), 1_000);
    assert_eq!(entry.id(), id);
}

#[test]
fn the_original_is_one_click_away() {
    let mut panel = ProvenancePanel::new();
    let id = record(
        &mut panel,
        "reflow-size",
        "reading-profile",
        SettingKey::ReflowSizePt,
        OriginalValue::setting(SettingValue::Int(14)),
    );

    assert_eq!(
        panel.original(id),
        Some(&OriginalValue::setting(SettingValue::Int(14)))
    );
}

#[test]
fn entries_are_oldest_first() {
    let mut panel = ProvenancePanel::new();
    let first = record(
        &mut panel,
        "dim-on",
        "focus-mode",
        SettingKey::DimEnabled,
        OriginalValue::None,
    );
    let second = record(
        &mut panel,
        "dim-off",
        "focus-mode",
        SettingKey::DimEnabled,
        OriginalValue::None,
    );

    assert_eq!(panel.len(), 2);
    let ids: Vec<u64> = panel.entries().iter().map(|e| e.id().get()).collect();
    assert_eq!(ids, vec![first.get(), second.get()]);
    assert_eq!(panel.entries()[0].action().as_str(), "dim-on");
    assert_eq!(panel.entries()[1].action().as_str(), "dim-off");
}

#[test]
fn for_action_filters() {
    let mut panel = ProvenancePanel::new();
    record(
        &mut panel,
        "dim-on",
        "focus-mode",
        SettingKey::DimEnabled,
        OriginalValue::None,
    );
    record(
        &mut panel,
        "dim-off",
        "calm-mode",
        SettingKey::DimEnabled,
        OriginalValue::None,
    );
    record(
        &mut panel,
        "dim-on",
        "focus-mode",
        SettingKey::DimEnabled,
        OriginalValue::None,
    );

    let matches: Vec<&str> = panel
        .for_action(&action("dim-on"))
        .map(|entry| entry.rule().0.as_str())
        .collect();
    assert_eq!(matches, vec!["focus-mode", "focus-mode"]);
    assert_eq!(panel.for_action(&action("dim-off")).count(), 1);
    assert_eq!(panel.for_action(&action("never-ran")).count(), 0);
}

#[test]
fn for_rule_filters() {
    let mut panel = ProvenancePanel::new();
    record(
        &mut panel,
        "dim-on",
        "focus-mode",
        SettingKey::DimEnabled,
        OriginalValue::None,
    );
    record(
        &mut panel,
        "dim-off",
        "calm-mode",
        SettingKey::DimEnabled,
        OriginalValue::None,
    );
    record(
        &mut panel,
        "motion-stop",
        "focus-mode",
        SettingKey::MotionStop,
        OriginalValue::None,
    );

    assert_eq!(panel.for_rule(&rule("focus-mode")).count(), 2);
    assert_eq!(panel.for_rule(&rule("calm-mode")).count(), 1);
    assert_eq!(panel.for_rule(&rule("unknown-rule")).count(), 0);
}

#[test]
fn content_originals_keep_no_text() {
    let original = OriginalValue::content("alpha-bravo-charlie");

    match original {
        OriginalValue::Content { len, .. } => assert_eq!(len, 19),
        other => panic!("expected content, got {other:?}"),
    }
    assert!(!original.describe().contains("alpha-bravo-charlie"));
    assert_eq!(original.describe(), "content (19 bytes)");
}

#[test]
fn content_hashes_are_stable_and_distinct() {
    let a = OriginalValue::content("alpha-bravo-charlie");
    let b = OriginalValue::content("alpha-bravo-charlie");
    let c = OriginalValue::content("delta-echo-foxtrot");

    assert_eq!(a, b, "the same text hashes the same");
    assert_ne!(a, c, "different text hashes differently");
}

#[test]
fn describe_is_log_safe() {
    let mut panel = ProvenancePanel::new();
    record(
        &mut panel,
        "simplify",
        "reading-level",
        SettingKey::ReadingLevel,
        OriginalValue::content("alpha-bravo-charlie"),
    );

    let entry = &panel.entries()[0];
    let summary = format!(
        "{} {} {}",
        entry.action().as_str(),
        entry.rule().0.as_str(),
        entry.original().describe()
    );

    assert_eq!(summary, "simplify reading-level content (19 bytes)");
    assert!(!summary.contains("alpha-bravo-charlie"));
}

#[test]
fn a_missing_entry_is_none() {
    let panel = ProvenancePanel::new();
    assert_eq!(panel.get(ProvenanceId(0)), None);
    assert_eq!(panel.original(ProvenanceId(0)), None);
    assert_eq!(panel.len(), 0);
    assert!(panel.is_empty());
}

#[test]
fn ids_are_unique() {
    let mut panel = ProvenancePanel::new();
    let first = record(
        &mut panel,
        "dim-on",
        "focus-mode",
        SettingKey::DimEnabled,
        OriginalValue::None,
    );
    let second = record(
        &mut panel,
        "dim-off",
        "focus-mode",
        SettingKey::DimEnabled,
        OriginalValue::None,
    );

    assert_ne!(first, second);
    assert_eq!(first.get(), 0);
    assert_eq!(second.get(), 1);
}

#[test]
fn clear_forgets_everything() {
    let mut panel = ProvenancePanel::new();
    record(
        &mut panel,
        "dim-on",
        "focus-mode",
        SettingKey::DimEnabled,
        OriginalValue::None,
    );
    assert_eq!(panel.len(), 1);

    panel.clear();

    assert!(panel.is_empty());
    assert!(panel.entries().is_empty());
    let next = record(
        &mut panel,
        "dim-on",
        "focus-mode",
        SettingKey::DimEnabled,
        OriginalValue::None,
    );
    assert_eq!(next.get(), 0, "ids restart after a clear");
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/PROVENANCE.md");
    for word in ["rule", "original", "never", "content"] {
        assert!(
            doc.contains(word),
            "docs/PROVENANCE.md does not state `{word}`"
        );
    }
}
