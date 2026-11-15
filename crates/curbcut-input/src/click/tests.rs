//! Tests for click-through routing.

use super::*;

#[test]
fn a_click_on_the_overlay_is_consumed() {
    let mut router = ClickRouter::new(500);

    assert_eq!(
        router.route(ClickTarget::Overlay, 0),
        ClickDecision::Consume
    );
    assert_eq!(
        router.route(ClickTarget::Overlay, 1_000),
        ClickDecision::Consume
    );
    assert_eq!(router.consume_count(), 2);
    assert_eq!(router.handoff_count(), 0);
    assert_eq!(router.suppressed_count(), 0);
    assert_eq!(router.focus_due(), None);
}

#[test]
fn a_click_on_the_app_is_handed_off() {
    let mut router = ClickRouter::new(500);

    assert_eq!(router.route(ClickTarget::App, 0), ClickDecision::Handoff);
    assert_eq!(router.handoff_count(), 1);
    let token = router.focus_due().expect("focus is owed");
    assert_eq!(token.get(), 0);
}

#[test]
fn handoff_is_never_accidental() {
    let mut router = ClickRouter::new(500);

    // Overlay clicks never hand off, no matter how many.
    for at_ms in [0, 1, 2, 3, 4] {
        assert_eq!(
            router.route(ClickTarget::Overlay, at_ms),
            ClickDecision::Consume
        );
    }
    assert_eq!(router.handoff_count(), 0);
    assert_eq!(router.focus_due(), None);

    // Only the app target hands off.
    assert_eq!(router.route(ClickTarget::App, 5), ClickDecision::Handoff);
    assert_eq!(router.handoff_count(), 1);
}

#[test]
fn a_second_handoff_inside_the_window_is_suppressed() {
    let mut router = ClickRouter::new(500);
    assert_eq!(router.route(ClickTarget::App, 0), ClickDecision::Handoff);
    let first = router.focus_due().copied().expect("focus is owed");

    assert_eq!(router.route(ClickTarget::App, 100), ClickDecision::Consume);
    assert_eq!(router.suppressed_count(), 1);
    assert_eq!(router.handoff_count(), 1);
    // The owed focus is unchanged: the suppressed click is not a second handoff.
    assert_eq!(router.focus_due(), Some(&first));
}

#[test]
fn a_handoff_after_the_window_is_allowed() {
    let mut router = ClickRouter::new(500);
    assert_eq!(router.route(ClickTarget::App, 0), ClickDecision::Handoff);

    // Exactly at the window boundary is not "inside" it.
    assert_eq!(router.route(ClickTarget::App, 500), ClickDecision::Handoff);
    assert_eq!(router.handoff_count(), 2);
    assert_eq!(router.suppressed_count(), 0);
}

#[test]
fn a_zero_window_never_suppresses() {
    let mut router = ClickRouter::new(0);
    assert_eq!(router.route(ClickTarget::App, 0), ClickDecision::Handoff);
    assert_eq!(router.route(ClickTarget::App, 0), ClickDecision::Handoff);
    assert_eq!(router.route(ClickTarget::App, 1), ClickDecision::Handoff);
    assert_eq!(router.handoff_count(), 3);
    assert_eq!(router.suppressed_count(), 0);
}

#[test]
fn focus_returns_and_clears() {
    let mut router = ClickRouter::new(500);
    assert_eq!(router.route(ClickTarget::App, 0), ClickDecision::Handoff);
    let owed = router.focus_due().copied().expect("focus is owed");

    let returned = router.return_focus().expect("focus was owed");
    assert_eq!(returned, owed);
    assert_eq!(router.focus_due(), None);
    assert_eq!(router.return_focus(), None);
}

#[test]
fn tokens_are_unique() {
    let mut router = ClickRouter::new(0);
    assert_eq!(router.route(ClickTarget::App, 0), ClickDecision::Handoff);
    let first = router.return_focus().expect("first");
    assert_eq!(router.route(ClickTarget::App, 1), ClickDecision::Handoff);
    let second = router.return_focus().expect("second");

    assert_ne!(first, second);
    assert_eq!(first.get(), 0);
    assert_eq!(second.get(), 1);
}

#[test]
fn time_going_backwards_is_suppressed() {
    let mut router = ClickRouter::new(500);
    assert_eq!(
        router.route(ClickTarget::App, 1_000),
        ClickDecision::Handoff
    );
    assert_eq!(router.route(ClickTarget::App, 500), ClickDecision::Consume);
    assert_eq!(router.suppressed_count(), 1);
    assert_eq!(router.handoff_count(), 1);
}

#[test]
fn reset_clears_everything() {
    let mut router = ClickRouter::new(500);
    assert_eq!(router.route(ClickTarget::App, 0), ClickDecision::Handoff);
    assert_eq!(router.route(ClickTarget::App, 100), ClickDecision::Consume);

    router.reset();

    assert_eq!(router.consume_count(), 0);
    assert_eq!(router.handoff_count(), 0);
    assert_eq!(router.suppressed_count(), 0);
    assert_eq!(router.focus_due(), None);
    // The window is forgotten, so the next app click is a handoff.
    assert_eq!(router.route(ClickTarget::App, 100), ClickDecision::Handoff);
    assert_eq!(router.focus_due().map(|token| token.get()), Some(0));
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/CLICKS.md");
    for sentence in [
        "Handoff is never accidental",
        "double-click",
        "focus",
        "never",
    ] {
        assert!(
            doc.contains(sentence),
            "docs/CLICKS.md does not state `{sentence}`"
        );
    }
}
