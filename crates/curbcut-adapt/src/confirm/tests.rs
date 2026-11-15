//! Tests for confirmation and undo.

use curbcut_core::id::RuleId;

use super::*;

fn action(slug: &str) -> ActionId {
    ActionId::new(slug)
}

fn rule(slug: &str) -> RuleId {
    RuleId::new(slug)
}

#[test]
fn action_ids_are_lowercased_and_displayable() {
    let id = action("Delete_Profile");
    assert_eq!(id.as_str(), "delete_profile");
    assert_eq!(id.to_string(), "delete_profile");
}

#[test]
fn a_confirmation_states_the_consequence() {
    let confirmation = Confirmation::new(
        action("delete_profile"),
        "The profile and its saved layouts are removed.",
        false,
    );

    assert_eq!(confirmation.action(), &action("delete_profile"));
    assert_eq!(
        confirmation.consequence(),
        "The profile and its saved layouts are removed."
    );
    assert!(!confirmation.is_undoable());
    assert!(!confirmation.offers_undo());
}

#[test]
fn undo_is_offered_only_when_possible() {
    let undoable = Confirmation::new(action("dim"), "The screen is dimmed.", true);
    assert!(undoable.is_undoable());
    assert!(undoable.offers_undo());

    let permanent = Confirmation::new(action("delete"), "The file is deleted.", false);
    assert!(!permanent.offers_undo());
}

#[test]
fn recording_a_confirmation_stores_the_outcome_and_rule() {
    let mut log = ActionLog::new();
    let confirmation = Confirmation::new(action("dim"), "The screen is dimmed.", true);

    log.record(&confirmation, rule("dim.toggle"), Decision::Confirmed);

    assert_eq!(log.len(), 1);
    let record = log.last().expect("one record");
    assert_eq!(record.action(), &action("dim"));
    assert_eq!(record.rule(), &rule("dim.toggle"));
    assert_eq!(record.outcome(), Outcome::Applied);
    assert!(record.is_undoable());
    assert!(!record.is_undone());
    assert_eq!(log.applied_count(), 1);
    assert_eq!(log.undone_count(), 0);
}

#[test]
fn a_dismissed_confirmation_is_recorded_as_dismissed() {
    let mut log = ActionLog::new();
    let confirmation = Confirmation::new(action("delete"), "The file is deleted.", false);

    log.record(&confirmation, rule("file.delete"), Decision::Dismissed);

    assert_eq!(
        log.last().expect("one record").outcome(),
        Outcome::Dismissed
    );
    assert_eq!(log.applied_count(), 0);
    assert!(!log.can_undo());
}

#[test]
fn undo_last_returns_none_on_an_empty_log() {
    let mut log = ActionLog::new();
    assert_eq!(log.undo_last(), None);
    assert!(!log.can_undo());
}

#[test]
fn undo_last_skips_a_dismissed_action() {
    let mut log = ActionLog::new();
    log.record(
        &Confirmation::new(action("delete"), "The file is deleted.", true),
        rule("file.delete"),
        Decision::Dismissed,
    );

    assert_eq!(log.undo_last(), None);
    assert_eq!(
        log.last().expect("one record").outcome(),
        Outcome::Dismissed
    );
}

#[test]
fn undo_last_refuses_a_non_undoable_action() {
    let mut log = ActionLog::new();
    log.record(
        &Confirmation::new(action("delete"), "The file is deleted.", false),
        rule("file.delete"),
        Decision::Confirmed,
    );

    assert_eq!(log.undo_last(), None);
    assert_eq!(log.last().expect("one record").outcome(), Outcome::Applied);
    assert_eq!(log.applied_count(), 1);
}

#[test]
fn undo_last_undoes_the_most_recent_undoable_action() {
    let mut log = ActionLog::new();
    log.record(
        &Confirmation::new(action("dim"), "The screen is dimmed.", true),
        rule("dim.toggle"),
        Decision::Confirmed,
    );
    log.record(
        &Confirmation::new(action("grid"), "The icon grid is shown.", true),
        rule("grid.toggle"),
        Decision::Confirmed,
    );
    log.record(
        &Confirmation::new(action("delete"), "The file is deleted.", false),
        rule("file.delete"),
        Decision::Confirmed,
    );

    assert_eq!(log.undo_last(), Some(action("grid")));
    assert_eq!(log.undone_count(), 1);
    assert_eq!(log.applied_count(), 2);
    assert!(log.records()[1].is_undone());
    assert!(!log.records()[0].is_undone());
    assert!(!log.records()[2].is_undone());

    // The permanent action cannot be undone, so the dim is next.
    assert_eq!(log.undo_last(), Some(action("dim")));
    assert_eq!(log.undone_count(), 2);
    assert!(!log.can_undo());
    assert_eq!(log.undo_last(), None);
}

#[test]
fn an_undone_action_is_not_undone_twice() {
    let mut log = ActionLog::new();
    log.record(
        &Confirmation::new(action("dim"), "The screen is dimmed.", true),
        rule("dim.toggle"),
        Decision::Confirmed,
    );

    assert_eq!(log.undo_last(), Some(action("dim")));
    assert_eq!(log.undo_last(), None);
    assert_eq!(log.undone_count(), 1);
}

#[test]
fn records_are_oldest_first() {
    let mut log = ActionLog::new();
    log.record(
        &Confirmation::new(action("dim"), "The screen is dimmed.", true),
        rule("dim.toggle"),
        Decision::Confirmed,
    );
    log.record(
        &Confirmation::new(action("grid"), "The icon grid is shown.", true),
        rule("grid.toggle"),
        Decision::Dismissed,
    );

    let actions: Vec<_> = log
        .records()
        .iter()
        .map(|record| record.action().clone())
        .collect();
    assert_eq!(actions, vec![action("dim"), action("grid")]);
    assert_eq!(log.last().map(ActionRecord::action), Some(&action("grid")));
}

#[test]
fn last_is_the_most_recent_record_even_after_an_undo() {
    let mut log = ActionLog::new();
    log.record(
        &Confirmation::new(action("dim"), "The screen is dimmed.", true),
        rule("dim.toggle"),
        Decision::Confirmed,
    );
    log.record(
        &Confirmation::new(action("grid"), "The icon grid is shown.", true),
        rule("grid.toggle"),
        Decision::Confirmed,
    );

    assert_eq!(log.undo_last(), Some(action("grid")));
    // The record is still the most recent; only its outcome changed.
    let last = log.last().expect("two records");
    assert_eq!(last.action(), &action("grid"));
    assert!(last.is_undone());
    assert_eq!(log.last().map(ActionRecord::action), Some(&action("grid")));
}

#[test]
fn clear_empties_the_log() {
    let mut log = ActionLog::new();
    log.record(
        &Confirmation::new(action("dim"), "The screen is dimmed.", true),
        rule("dim.toggle"),
        Decision::Confirmed,
    );

    log.clear();

    assert!(log.is_empty());
    assert_eq!(log.len(), 0);
    assert_eq!(log.records(), &[]);
    assert_eq!(log.last(), None);
    assert!(!log.can_undo());
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/CONFIRM.md");
    for sentence in [
        "The consequence is stated.",
        "Undo is offered only when possible.",
        "The outcome is recorded.",
        "per-action opt-in",
        "never holds screen",
    ] {
        assert!(
            doc.contains(sentence),
            "docs/CONFIRM.md does not state `{sentence}`"
        );
    }
}
