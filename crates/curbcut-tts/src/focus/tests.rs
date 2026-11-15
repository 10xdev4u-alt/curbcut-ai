//! Tests for audio focus.

use super::*;

#[test]
fn focus_starts_idle() {
    let focus = AudioFocus::new();
    assert_eq!(focus.state(), FocusState::Idle);
    assert!(!focus.is_granted());
    assert_eq!(AudioFocus::default().state(), FocusState::Idle);
}

#[test]
fn focus_is_requested_then_granted() {
    let mut focus = AudioFocus::new();

    assert_eq!(focus.request(), FocusState::Requested);
    assert!(!focus.is_granted());
    assert_eq!(focus.grant(), FocusState::Granted);
    assert!(focus.is_granted());
}

#[test]
fn a_grant_without_a_request_is_ignored() {
    let mut focus = AudioFocus::new();
    assert_eq!(focus.grant(), FocusState::Idle);
    assert!(!focus.is_granted());
}

#[test]
fn a_second_grant_changes_nothing() {
    let mut focus = AudioFocus::new();
    focus.request();
    focus.grant();

    assert_eq!(focus.grant(), FocusState::Granted);
    assert!(focus.is_granted());
}

#[test]
fn abandoning_granted_focus_returns_to_idle() {
    let mut focus = AudioFocus::new();
    focus.request();
    focus.grant();

    assert_eq!(focus.abandon(), FocusState::Idle);
    assert!(!focus.is_granted());
    assert_eq!(focus.abandon(), FocusState::Idle);
}

#[test]
fn abandoning_a_request_marks_it_abandoned() {
    let mut focus = AudioFocus::new();
    focus.request();

    assert_eq!(focus.abandon(), FocusState::Abandoned);
    assert!(!focus.is_granted());
}

#[test]
fn a_request_after_abandoning_starts_again() {
    let mut focus = AudioFocus::new();
    focus.request();
    focus.abandon();

    assert_eq!(focus.request(), FocusState::Requested);
    assert_eq!(focus.grant(), FocusState::Granted);
}

#[test]
fn reset_returns_to_idle() {
    let mut focus = AudioFocus::new();
    focus.request();
    focus.grant();

    focus.reset();

    assert_eq!(focus.state(), FocusState::Idle);
    assert!(!focus.is_granted());
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/SPEECH.md");
    for word in ["voice", "opt-in", "fallback", "network", "audio"] {
        assert!(doc.contains(word), "docs/SPEECH.md does not state `{word}`");
    }
}
