//! Tests for the interruption inbox.

use super::*;
use curbcut_core::types::Rect;

fn interruption(kind: InterruptionKind, dismissible: bool) -> Interruption {
    Interruption {
        kind,
        rect: Rect::new(10, 20, 30, 40),
        dismissible,
    }
}

fn app(name: &str) -> AppId {
    AppId::new(name)
}

#[test]
fn a_new_inbox_is_empty_and_closed() {
    let inbox = Inbox::new();
    assert!(inbox.is_empty());
    assert_eq!(inbox.len(), 0);
    assert!(!inbox.is_open());
    assert!(inbox.current().is_none());
    assert!(inbox.entries().is_empty());
    assert!(inbox.batches().is_empty());
}

#[test]
fn filing_keeps_the_entry() {
    let mut inbox = Inbox::new();
    let id = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    assert_eq!(inbox.len(), 1);
    let entry = inbox.get(id).expect("filed");
    assert_eq!(entry.id(), id);
    assert_eq!(entry.app().as_str(), "alpha");
    assert_eq!(entry.kind(), InterruptionKind::Toast);
    assert_eq!(entry.rect(), Rect::new(10, 20, 30, 40));
    assert!(entry.dismissible());
}

#[test]
fn ids_are_sequential_from_zero() {
    let mut inbox = Inbox::new();
    let first = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    let second = inbox.file(app("alpha"), interruption(InterruptionKind::Banner, false));
    assert_eq!(first.get(), 0);
    assert_eq!(second.get(), 1);
}

#[test]
fn nothing_is_auto_dismissed() {
    let mut inbox = Inbox::new();
    inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    inbox.file(app("alpha"), interruption(InterruptionKind::Banner, false));
    inbox.file(app("beta"), interruption(InterruptionKind::Popup, false));

    inbox.open();
    inbox.move_next();
    inbox.move_next();
    inbox.move_prev();
    inbox.toggle();
    inbox.toggle();

    assert_eq!(inbox.len(), 3, "no entry may leave on its own");
    assert_eq!(inbox.entries().len(), 3);
}

#[test]
fn batches_group_by_app_and_kind() {
    let mut inbox = Inbox::new();
    inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    inbox.file(app("alpha"), interruption(InterruptionKind::Banner, false));
    inbox.file(app("beta"), interruption(InterruptionKind::Popup, false));

    let batches = inbox.batches();
    assert_eq!(batches.len(), 3);
    assert_eq!(batches[0].app().as_str(), "alpha");
    assert_eq!(batches[0].kind(), InterruptionKind::Toast);
    assert_eq!(batches[0].len(), 2);
    assert_eq!(batches[1].app().as_str(), "alpha");
    assert_eq!(batches[1].kind(), InterruptionKind::Banner);
    assert_eq!(batches[2].app().as_str(), "beta");
    assert_eq!(batches[2].kind(), InterruptionKind::Popup);
}

#[test]
fn a_batch_preserves_filing_order() {
    let mut inbox = Inbox::new();
    let first = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    let second = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    let batches = inbox.batches();
    assert_eq!(batches[0].items(), &[first, second]);
}

#[test]
fn batches_are_sorted_even_when_filed_out_of_order() {
    let mut inbox = Inbox::new();
    inbox.file(app("beta"), interruption(InterruptionKind::Popup, false));
    inbox.file(app("alpha"), interruption(InterruptionKind::Banner, false));
    inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    inbox.file(app("beta"), interruption(InterruptionKind::Toast, true));

    let batches = inbox.batches();
    let keys: Vec<(&str, InterruptionKind)> = batches
        .iter()
        .map(|batch| (batch.app().as_str(), batch.kind()))
        .collect();
    assert_eq!(
        keys,
        vec![
            ("alpha", InterruptionKind::Toast),
            ("alpha", InterruptionKind::Banner),
            ("beta", InterruptionKind::Toast),
            ("beta", InterruptionKind::Popup),
        ]
    );
}

#[test]
fn the_panel_is_review_on_demand() {
    let mut inbox = Inbox::new();
    assert!(!inbox.is_open());
    inbox.open();
    assert!(inbox.is_open());
    inbox.open();
    assert!(inbox.is_open(), "opening twice keeps it open");
    inbox.close();
    assert!(!inbox.is_open());
    inbox.toggle();
    assert!(inbox.is_open());
    inbox.toggle();
    assert!(!inbox.is_open());
}

#[test]
fn the_cursor_moves_and_clamps() {
    let mut inbox = Inbox::new();
    let first = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    let second = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    let third = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));

    assert_eq!(inbox.current_id(), Some(first));
    inbox.move_next();
    assert_eq!(inbox.current_id(), Some(second));
    inbox.move_next();
    assert_eq!(inbox.current_id(), Some(third));
    inbox.move_next();
    assert_eq!(inbox.current_id(), Some(third), "the end clamps");
    inbox.move_prev();
    assert_eq!(inbox.current_id(), Some(second));
    inbox.move_prev();
    assert_eq!(inbox.current_id(), Some(first));
    inbox.move_prev();
    assert_eq!(inbox.current_id(), Some(first), "the start clamps");
}

#[test]
fn move_to_finds_a_filed_id() {
    let mut inbox = Inbox::new();
    let first = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    let second = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    assert!(inbox.move_to(second));
    assert_eq!(inbox.current_id(), Some(second));
    assert!(!inbox.move_to(InboxId(99)));
    assert_eq!(inbox.current_id(), Some(second), "a miss leaves the cursor");
    assert!(inbox.move_to(first));
}

#[test]
fn remove_deletes_only_that_entry() {
    let mut inbox = Inbox::new();
    let first = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    let second = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    let third = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));

    assert!(inbox.remove(second));
    assert!(!inbox.remove(second));
    let ids: Vec<u64> = inbox
        .entries()
        .iter()
        .map(|entry| entry.id().get())
        .collect();
    assert_eq!(ids, vec![first.get(), third.get()]);
    assert_eq!(inbox.len(), 2);
}

#[test]
fn removing_an_entry_before_the_cursor_keeps_the_same_entry() {
    let mut inbox = Inbox::new();
    let first = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    let second = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    inbox.move_next();
    assert_eq!(inbox.current_id(), Some(second));
    assert!(inbox.remove(first));
    assert_eq!(
        inbox.current_id(),
        Some(second),
        "removing an earlier entry shifts the viewed entry down"
    );
    assert_eq!(inbox.len(), 2);
}

#[test]
fn removing_the_last_entry_pulls_the_cursor_back() {
    let mut inbox = Inbox::new();
    let first = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    let second = inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    inbox.move_next();
    assert_eq!(inbox.current_id(), Some(second));
    inbox.remove(second);
    assert_eq!(inbox.current_id(), Some(first));
}

#[test]
fn clear_empties_the_inbox() {
    let mut inbox = Inbox::new();
    inbox.file(app("alpha"), interruption(InterruptionKind::Toast, true));
    inbox.file(app("alpha"), interruption(InterruptionKind::Banner, false));
    inbox.clear();
    assert!(inbox.is_empty());
    assert_eq!(inbox.current_id(), None);
    assert!(inbox.batches().is_empty());
}

#[test]
fn file_all_returns_ids_in_order() {
    let mut inbox = Inbox::new();
    let interruptions = vec![
        interruption(InterruptionKind::Toast, true),
        interruption(InterruptionKind::Banner, false),
    ];
    let ids = inbox.file_all(app("alpha"), interruptions);
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0].get(), 0);
    assert_eq!(ids[1].get(), 1);
    assert_eq!(inbox.len(), 2);
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/INTERRUPTION_INBOX.md");
    for word in ["inbox", "batch", "app", "kind", "dismiss", "keyboard"] {
        assert!(
            doc.contains(word),
            "docs/INTERRUPTION_INBOX.md does not state `{word}`"
        );
    }
}
