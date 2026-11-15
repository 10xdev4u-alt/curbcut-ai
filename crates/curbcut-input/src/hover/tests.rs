//! Tests for pointer tracking.

use curbcut_core::types::{Point, Rect};

use super::*;

fn sample(x: i32, y: i32, at_ms: u64) -> PointerSample {
    PointerSample::new(Point::new(x, y), at_ms)
}

#[test]
fn a_single_sample_is_unknown() {
    let mut tracker = HoverTracker::new(100, 5);
    assert_eq!(tracker.state(), PointerState::Unknown);
    assert!(tracker.state().is_unknown());

    tracker.update(sample(10, 10, 0));

    assert_eq!(tracker.position(), Some(Point::new(10, 10)));
    assert_eq!(tracker.state(), PointerState::Unknown);
    assert_eq!(tracker.dwell_ms(), 0);
    assert_eq!(tracker.sample_count(), 1);
    assert_eq!(tracker.move_count(), 0);
    assert!(tracker.is_over(Rect::new(0, 0, 20, 20)));
}

#[test]
fn a_pointer_that_stays_put_settles() {
    let mut tracker = HoverTracker::new(100, 5);
    tracker.update(sample(10, 10, 0));
    tracker.update(sample(10, 10, 150));

    assert_eq!(tracker.state(), PointerState::Settled);
    assert!(tracker.state().is_settled());
    assert_eq!(tracker.dwell_ms(), 150);
}

#[test]
fn a_pointer_inside_the_window_is_moving() {
    let mut tracker = HoverTracker::new(100, 5);
    tracker.update(sample(10, 10, 0));
    tracker.update(sample(10, 10, 50));

    assert_eq!(tracker.state(), PointerState::Moving);
    assert!(tracker.state().is_moving());
    assert_eq!(tracker.dwell_ms(), 50);
}

#[test]
fn a_move_beyond_the_threshold_resets_the_dwell() {
    let mut tracker = HoverTracker::new(100, 5);
    tracker.update(sample(0, 0, 0));
    tracker.update(sample(0, 0, 50));
    assert_eq!(tracker.dwell_ms(), 50);

    tracker.update(sample(10, 0, 100));

    assert_eq!(tracker.state(), PointerState::Moving);
    assert_eq!(tracker.dwell_ms(), 0);
    assert_eq!(tracker.move_count(), 1);
}

#[test]
fn a_move_within_the_threshold_keeps_the_dwell() {
    let mut tracker = HoverTracker::new(100, 5);
    tracker.update(sample(0, 0, 0));
    tracker.update(sample(3, 0, 50));

    assert_eq!(tracker.state(), PointerState::Moving);
    assert_eq!(tracker.dwell_ms(), 50);
    assert_eq!(tracker.move_count(), 0);
}

#[test]
fn a_diagonal_drift_inside_the_box_is_not_a_move() {
    let mut tracker = HoverTracker::new(100, 5);
    tracker.update(sample(0, 0, 0));
    tracker.update(sample(3, 3, 50));

    assert_eq!(tracker.move_count(), 0);
    assert_eq!(tracker.dwell_ms(), 50);
}

#[test]
fn a_diagonal_drift_past_the_threshold_is_a_move() {
    let mut tracker = HoverTracker::new(100, 5);
    tracker.update(sample(0, 0, 0));
    tracker.update(sample(6, 0, 50));

    assert_eq!(tracker.move_count(), 1);
    assert_eq!(tracker.dwell_ms(), 0);
}

#[test]
fn a_negative_threshold_is_treated_as_zero() {
    let tracker = HoverTracker::new(100, -5);
    assert_eq!(tracker.move_threshold(), 0);
}

#[test]
fn move_count_and_path_length_accumulate() {
    let mut tracker = HoverTracker::new(1_000, 5);
    tracker.update(sample(0, 0, 0));
    tracker.update(sample(10, 0, 10));
    tracker.update(sample(10, 10, 20));

    assert_eq!(tracker.move_count(), 2);
    assert_eq!(tracker.path_length_px(), 20);
    assert_eq!(tracker.sample_count(), 3);
}

#[test]
fn a_small_drift_accumulates_path_length_without_counting_as_a_move() {
    let mut tracker = HoverTracker::new(1_000, 5);
    tracker.update(sample(0, 0, 0));
    tracker.update(sample(3, 0, 10));
    tracker.update(sample(3, 4, 20));

    assert_eq!(tracker.move_count(), 0);
    assert_eq!(tracker.path_length_px(), 7);
}

#[test]
fn is_over_uses_the_last_position() {
    let mut tracker = HoverTracker::new(100, 5);
    let rect = Rect::new(0, 0, 10, 10);

    tracker.update(sample(5, 5, 0));
    assert!(tracker.is_over(rect));

    tracker.update(sample(20, 5, 10));
    assert!(!tracker.is_over(rect));
}

#[test]
fn is_over_is_half_open() {
    let mut tracker = HoverTracker::new(100, 5);
    let rect = Rect::new(0, 0, 10, 10);

    tracker.update(sample(0, 0, 0));
    assert!(tracker.is_over(rect));

    tracker.update(sample(9, 9, 10));
    assert!(tracker.is_over(rect));

    tracker.update(sample(10, 5, 20));
    assert!(!tracker.is_over(rect), "the right edge is outside");

    tracker.update(sample(5, 10, 30));
    assert!(!tracker.is_over(rect), "the bottom edge is outside");
}

#[test]
fn a_zero_settle_window_settles_immediately() {
    let mut tracker = HoverTracker::new(0, 5);
    tracker.update(sample(10, 10, 0));
    tracker.update(sample(10, 10, 0));

    assert_eq!(tracker.state(), PointerState::Settled);
    assert_eq!(tracker.dwell_ms(), 0);
}

#[test]
fn time_going_backwards_does_not_panic() {
    let mut tracker = HoverTracker::new(100, 5);
    tracker.update(sample(0, 0, 100));
    tracker.update(sample(0, 0, 50));

    assert_eq!(tracker.dwell_ms(), 0);
    assert_eq!(tracker.state(), PointerState::Moving);
    assert_eq!(tracker.move_count(), 0);
}

#[test]
fn extreme_coordinates_do_not_overflow() {
    let mut tracker = HoverTracker::new(100, 5);
    tracker.update(sample(i32::MIN, i32::MIN, 0));
    tracker.update(sample(i32::MAX, i32::MAX, 10));

    let span = i64::from(i32::MAX) - i64::from(i32::MIN);
    assert_eq!(tracker.path_length_px(), (span * 2) as u64);
    assert_eq!(tracker.move_count(), 1);
}

#[test]
fn reset_forgets_everything() {
    let mut tracker = HoverTracker::new(100, 5);
    tracker.update(sample(10, 10, 0));
    tracker.update(sample(10, 10, 150));
    assert_eq!(tracker.state(), PointerState::Settled);

    tracker.reset();

    assert_eq!(tracker.position(), None);
    assert_eq!(tracker.state(), PointerState::Unknown);
    assert_eq!(tracker.dwell_ms(), 0);
    assert_eq!(tracker.sample_count(), 0);
    assert_eq!(tracker.move_count(), 0);
    assert_eq!(tracker.path_length_px(), 0);
    assert!(!tracker.is_over(Rect::new(0, 0, 20, 20)));
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/HOVER.md");
    for word in ["Wayland", "settle", "never", "hover"] {
        assert!(doc.contains(word), "docs/HOVER.md does not state `{word}`");
    }
}
