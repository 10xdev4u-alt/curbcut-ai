//! Tests for the right-to-explanation report.

use curbcut_core::id::RuleId;
use curbcut_core::types::Rationale;

use super::*;
use crate::confirm::{ActionId, ActionLog, Confirmation, Decision, Outcome};
use crate::provenance::{OriginalValue, ProvenancePanel};
use crate::settings::{SettingKey, SettingValue};

fn rule(slug: &str) -> RuleId {
    RuleId(slug.to_string())
}

fn action(slug: &str) -> ActionId {
    ActionId::new(slug)
}

/// A panel with one entry per `(action, rule, original, time)`.
fn panel(entries: &[(&str, &str, OriginalValue, u64)]) -> ProvenancePanel {
    let mut panel = ProvenancePanel::new();
    for (action_slug, rule_slug, original, time) in entries {
        panel.record(
            action(action_slug),
            rule(rule_slug),
            SettingKey::DimOpacity,
            original.clone(),
            *time,
        );
    }
    panel
}

/// A log with one record per `(action, rule, outcome)`.
fn log(entries: &[(&str, &str, Outcome)]) -> ActionLog {
    let mut log = ActionLog::new();
    for (action_slug, rule_slug, outcome) in entries {
        let decision = match outcome {
            Outcome::Applied | Outcome::Undone => Decision::Confirmed,
            Outcome::Dismissed => Decision::Dismissed,
        };
        log.record(
            &Confirmation::new(action(action_slug), "a consequence", true),
            rule(rule_slug),
            decision,
        );
        if *outcome == Outcome::Undone {
            log.undo_last();
        }
    }
    log
}

fn rationales(entries: &[(&str, &str)]) -> Vec<Rationale> {
    entries
        .iter()
        .map(|(rule_slug, text)| Rationale::new(rule(rule_slug), *text))
        .collect()
}

#[test]
fn the_documented_rules_match_the_docs() {
    let doc = include_str!("../../../../docs/RIGHT_TO_EXPLANATION.md");
    for word in [
        "why",
        "original",
        "grade",
        "rationale",
        "redacted",
        "content",
        "action log",
    ] {
        assert!(
            doc.contains(word),
            "docs/RIGHT_TO_EXPLANATION.md does not state `{word}`"
        );
    }
}

#[test]
fn a_log_and_a_panel_are_paired_by_action_in_order() {
    let log = log(&[
        ("dim", "bright", Outcome::Applied),
        ("dim", "bright", Outcome::Undone),
        ("read", "long", Outcome::Applied),
    ]);
    let panel = panel(&[
        (
            "dim",
            "bright",
            OriginalValue::setting(SettingValue::Float(0.2)),
            10,
        ),
        (
            "dim",
            "bright",
            OriginalValue::setting(SettingValue::Float(0.4)),
            20,
        ),
        ("read", "long", OriginalValue::content("a long article"), 30),
    ]);
    let report = Report::from_sources(&log, &panel, &[], 99);

    assert_eq!(report.len(), 3);
    assert_eq!(report.entries()[0].action(), &action("dim"));
    assert_eq!(report.entries()[0].outcome(), Some(Outcome::Applied));
    assert_eq!(report.entries()[0].changed_at_ms(), Some(10));
    assert_eq!(report.entries()[1].outcome(), Some(Outcome::Undone));
    assert_eq!(report.entries()[1].changed_at_ms(), Some(20));
    assert_eq!(report.entries()[2].action(), &action("read"));
    assert_eq!(report.entries()[2].changed_at_ms(), Some(30));
}

#[test]
fn an_action_only_in_the_log_is_still_reported() {
    let log = log(&[("dim", "bright", Outcome::Applied)]);
    let report = Report::from_sources(&log, &ProvenancePanel::new(), &[], 1);

    assert_eq!(report.len(), 1);
    let entry = &report.entries()[0];
    assert_eq!(entry.outcome(), Some(Outcome::Applied));
    assert_eq!(entry.original(), &OriginalValue::None);
    assert_eq!(entry.changed_at_ms(), None);
    assert_eq!(entry.rule(), &rule("bright"));
}

#[test]
fn an_action_only_in_the_panel_is_still_reported() {
    let panel = panel(&[(
        "dim",
        "bright",
        OriginalValue::setting(SettingValue::Float(0.2)),
        42,
    )]);
    let report = Report::from_sources(&ActionLog::new(), &panel, &[], 1);

    assert_eq!(report.len(), 1);
    let entry = &report.entries()[0];
    assert_eq!(entry.outcome(), None);
    assert_eq!(entry.changed_at_ms(), Some(42));
    assert_eq!(
        entry.original(),
        &OriginalValue::setting(SettingValue::Float(0.2))
    );
}

#[test]
fn entries_follow_first_appearance_across_both_logs() {
    let log = log(&[("read", "long", Outcome::Applied)]);
    let panel = panel(&[
        ("dim", "bright", OriginalValue::None, 5),
        ("read", "long", OriginalValue::None, 6),
    ]);
    let report = Report::from_sources(&log, &panel, &[], 1);

    let actions: Vec<&ActionId> = report.entries().iter().map(ReportEntry::action).collect();
    assert_eq!(actions, vec![&action("read"), &action("dim")]);
}

#[test]
fn a_repeated_action_pairs_each_occurrence_in_order() {
    let log = log(&[
        ("dim", "bright", Outcome::Applied),
        ("dim", "bright", Outcome::Applied),
    ]);
    let panel = panel(&[
        (
            "dim",
            "bright",
            OriginalValue::setting(SettingValue::Float(0.1)),
            1,
        ),
        (
            "dim",
            "bright",
            OriginalValue::setting(SettingValue::Float(0.9)),
            2,
        ),
    ]);
    let report = Report::from_sources(&log, &panel, &[], 1);

    let originals: Vec<String> = report
        .entries()
        .iter()
        .map(|entry| entry.original().render())
        .collect();
    assert_eq!(originals, vec!["0.100".to_string(), "0.900".to_string()]);
}

#[test]
fn every_action_links_to_its_rationale() {
    let log = log(&[
        ("dim", "bright", Outcome::Applied),
        ("read", "long", Outcome::Applied),
    ]);
    let panel = ProvenancePanel::new();
    let rationales = rationales(&[(
        "bright",
        "The screen was brighter than your profile allows.",
    )]);

    let report = Report::from_sources(&log, &panel, &rationales, 1);
    assert_eq!(report.len(), 2);
    assert!(report.entries()[0].is_explained());
    assert!(!report.entries()[1].is_explained());
    assert_eq!(
        report.missing_rationale(),
        vec![&action("read")],
        "the unexplained action is named, not dropped"
    );
    assert!(!report.is_explained());
    assert!(report.entries()[0]
        .explanation()
        .contains("The screen was brighter"));
}

#[test]
fn an_empty_rationale_counts_as_missing() {
    let log = log(&[("dim", "bright", Outcome::Applied)]);
    let rationales = rationales(&[("bright", "   ")]);
    let report = Report::from_sources(&log, &ProvenancePanel::new(), &rationales, 1);

    assert!(!report.entries()[0].is_explained());
    assert_eq!(report.missing_rationale(), vec![&action("dim")]);
    assert!(report.entries()[0]
        .explanation()
        .contains("no reason was recorded"));
}

#[test]
fn the_first_rationale_for_a_rule_wins() {
    let log = log(&[("dim", "bright", Outcome::Applied)]);
    let rationales = rationales(&[("bright", "first"), ("bright", "second")]);
    let report = Report::from_sources(&log, &ProvenancePanel::new(), &rationales, 1);

    assert!(report.entries()[0].reason().unwrap().contains("first"));
}

#[test]
fn what_happened_names_each_outcome() {
    assert_eq!(
        ReportEntry::new(
            action("dim"),
            rule("bright"),
            Some(Outcome::Applied),
            OriginalValue::None,
            None,
            None
        )
        .what_happened(),
        "CurbCut applied"
    );
    assert_eq!(
        ReportEntry::new(
            action("dim"),
            rule("bright"),
            Some(Outcome::Dismissed),
            OriginalValue::None,
            None,
            None
        )
        .what_happened(),
        "You dismissed"
    );
    assert_eq!(
        ReportEntry::new(
            action("dim"),
            rule("bright"),
            Some(Outcome::Undone),
            OriginalValue::None,
            None,
            None
        )
        .what_happened(),
        "CurbCut applied, then undid"
    );
    assert_eq!(
        ReportEntry::new(
            action("dim"),
            rule("bright"),
            None,
            OriginalValue::None,
            None,
            None
        )
        .what_happened(),
        "The outcome of"
    );
}

#[test]
fn the_sentence_matches_the_outcome() {
    let entry = |outcome: Option<Outcome>| {
        ReportEntry::new(
            action("dim"),
            rule("bright"),
            outcome,
            OriginalValue::None,
            None,
            None,
        )
    };
    assert_eq!(
        entry(Some(Outcome::Applied)).sentence(),
        "CurbCut applied the dim action."
    );
    assert_eq!(
        entry(Some(Outcome::Dismissed)).sentence(),
        "You dismissed the dim action."
    );
    assert_eq!(
        entry(Some(Outcome::Undone)).sentence(),
        "CurbCut applied the dim action, then undid it."
    );
    assert_eq!(
        entry(None).sentence(),
        "The outcome of the dim action was not recorded."
    );
}

#[test]
fn a_content_original_is_rendered_without_its_text() {
    let secret = "MESSAGE-FROM-ANOTHER-APP-42";
    let original = OriginalValue::content(secret);
    let rendered = original.render();

    assert!(!rendered.contains(secret));
    assert!(rendered.contains("content"));
    assert!(rendered.contains(&secret.len().to_string()));
    assert_ne!(rendered, secret);
}

#[test]
fn a_setting_original_is_rendered_with_its_value() {
    assert_eq!(
        OriginalValue::setting(SettingValue::Bool(true)).render(),
        "on"
    );
    assert_eq!(
        OriginalValue::setting(SettingValue::Bool(false)).render(),
        "off"
    );
    assert_eq!(OriginalValue::setting(SettingValue::Int(7)).render(), "7");
    assert_eq!(
        OriginalValue::setting(SettingValue::Float(0.25)).render(),
        "0.250"
    );
    assert_eq!(
        OriginalValue::setting(SettingValue::Text("Focus".to_string())).render(),
        "Focus"
    );
    assert_eq!(
        OriginalValue::setting(SettingValue::TextList(vec!["work".into(), "home".into()])).render(),
        "work, home"
    );
    assert_eq!(OriginalValue::None.render(), "nothing was kept");
}

#[test]
fn describe_stays_log_safe_where_render_shows_the_value() {
    let original = OriginalValue::setting(SettingValue::Text("a profile name".to_string()));
    assert_eq!(original.describe(), "setting text");
    assert_eq!(original.render(), "a profile name");
}

#[test]
fn the_export_contains_no_other_apps_content() {
    let secret = "SCREEN-TEXT-7C1F";
    let log = log(&[("read", "long", Outcome::Applied)]);
    let panel = panel(&[("read", "long", OriginalValue::content(secret), 5)]);
    let report = Report::from_sources(&log, &panel, &rationales(&[("long", "It was long.")]), 7);

    let plain = report.to_plain_text();
    assert!(!plain.contains(secret), "the export leaked screen text");
    assert!(plain.contains("CurbCut action report"));
    assert!(plain.contains("Actions listed: 1"));
    assert!(plain.contains("1. CurbCut applied the read action."));
    assert!(plain.contains("Why: It was long."));
    assert!(plain.contains("Original: content, 16 bytes"));

    let markdown = report.to_markdown();
    assert!(!markdown.contains(secret));
    assert!(markdown.contains("# CurbCut action report"));
    assert!(markdown.contains("- CurbCut applied the read action."));
}

#[test]
fn the_report_measures_its_own_reading_level() {
    let log = log(&[("dim", "bright", Outcome::Applied)]);
    let rationales = rationales(&[("bright", "The screen was bright."), ("bright", "second")]);
    let report = Report::from_sources(&log, &ProvenancePanel::new(), &rationales, 1);

    let level = report.grade_level().expect("latin text is supported");
    assert!(level.is_finite());
    assert!(report.is_readable_at(12.0));
    assert!(!report.is_readable_at(-1.0));
    assert!(report.to_plain_text().contains(&format!("{level:.1}")));
}

#[test]
fn an_empty_report_is_explained_and_still_measures_its_boilerplate() {
    let report = Report::from_sources(&ActionLog::new(), &ProvenancePanel::new(), &[], 3);
    assert!(report.is_empty());
    assert_eq!(report.len(), 0);
    assert!(report.is_explained(), "no entry is missing a reason");
    assert!(report.missing_rationale().is_empty());
    assert_eq!(report.generated_at_ms(), 3);
    assert!(report.to_plain_text().contains("Actions listed: 0"));
    assert!(
        report.grade_level().is_some(),
        "the header alone is measurable text"
    );
    assert!(report.is_readable_at(12.0));
}

#[test]
fn entries_are_exposed_in_order() {
    let log = log(&[
        ("dim", "bright", Outcome::Applied),
        ("read", "long", Outcome::Dismissed),
    ]);
    let report = Report::from_sources(&log, &ProvenancePanel::new(), &[], 1);

    let actions: Vec<&ActionId> = report.entries().iter().map(ReportEntry::action).collect();
    assert_eq!(actions, vec![&action("dim"), &action("read")]);
    assert_eq!(report.entries()[1].outcome(), Some(Outcome::Dismissed));
}
