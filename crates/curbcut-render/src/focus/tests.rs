//! Tests for the reading ruler and focus lens.

use curbcut_core::types::{Point, Rect};

use super::*;

fn screen() -> Rect {
    Rect::new(0, 0, 100, 100)
}

fn wide_screen() -> Rect {
    Rect::new(0, 0, 400, 400)
}

#[test]
fn line_count_is_clamped_to_one_to_three() {
    assert_eq!(ReadingRuler::new(0, 10, MotionMode::Full).line_count(), 1);
    assert_eq!(ReadingRuler::new(1, 10, MotionMode::Full).line_count(), 1);
    assert_eq!(ReadingRuler::new(2, 10, MotionMode::Full).line_count(), 2);
    assert_eq!(ReadingRuler::new(3, 10, MotionMode::Full).line_count(), 3);
    assert_eq!(ReadingRuler::new(9, 10, MotionMode::Full).line_count(), 3);

    let mut ruler = ReadingRuler::new(2, 10, MotionMode::Full);
    ruler.set_line_count(0);
    assert_eq!(ruler.line_count(), 1);
    ruler.set_line_count(7);
    assert_eq!(ruler.line_count(), 3);
}

#[test]
fn band_height_is_lines_times_line_height() {
    assert_eq!(ReadingRuler::new(1, 10, MotionMode::Full).band_height(), 10);
    assert_eq!(ReadingRuler::new(2, 10, MotionMode::Full).band_height(), 20);
    assert_eq!(ReadingRuler::new(3, 12, MotionMode::Full).band_height(), 36);
}

#[test]
fn the_ruler_clamps_to_the_screen() {
    let mut ruler = ReadingRuler::new(3, 10, MotionMode::Full);
    assert_eq!(ruler.move_to(-50, screen()), Some(0));
    assert_eq!(ruler.move_to(200, screen()), Some(70));
    assert_eq!(ruler.move_to(40, screen()), Some(40));
}

#[test]
fn the_ruler_spans_the_screen_width() {
    let mut ruler = ReadingRuler::new(2, 10, MotionMode::Full);
    ruler.move_to(40, screen());

    assert_eq!(ruler.rect(screen()), Some(Rect::new(0, 40, 100, 20)));
}

#[test]
fn reduced_motion_hides_the_ruler() {
    let mut ruler = ReadingRuler::new(2, 10, MotionMode::Reduced);

    assert_eq!(ruler.rect(screen()), None);
    assert_eq!(ruler.move_to(40, screen()), None);

    ruler.set_motion(MotionMode::Full);
    assert_eq!(ruler.move_to(40, screen()), Some(40));
    assert_eq!(ruler.rect(screen()), Some(Rect::new(0, 40, 100, 20)));
}

#[test]
fn magnification_is_clamped() {
    assert_eq!(
        FocusLens::new(0.5, 0, 0, MotionMode::Full).magnification(),
        1.0
    );
    assert_eq!(
        FocusLens::new(1.0, 0, 0, MotionMode::Full).magnification(),
        1.0
    );
    assert_eq!(
        FocusLens::new(2.5, 0, 0, MotionMode::Full).magnification(),
        2.5
    );
    assert_eq!(
        FocusLens::new(99.0, 0, 0, MotionMode::Full).magnification(),
        8.0
    );

    let mut lens = FocusLens::new(2.0, 0, 0, MotionMode::Full);
    lens.set_magnification(0.1);
    assert_eq!(lens.magnification(), 1.0);
    lens.set_magnification(20.0);
    assert_eq!(lens.magnification(), 8.0);
}

#[test]
fn lead_and_dead_zone_are_clamped() {
    let lens = FocusLens::new(2.0, -5, -1, MotionMode::Full);
    assert_eq!(lens.lead_px(), 0);
    assert_eq!(lens.dead_zone_px(), 0);

    let lens = FocusLens::new(2.0, 500, 500, MotionMode::Full);
    assert_eq!(lens.lead_px(), 64);
    assert_eq!(lens.dead_zone_px(), 64);
}

#[test]
fn the_first_update_centres_on_the_pointer() {
    let mut lens = FocusLens::new(2.0, 10, 0, MotionMode::Full);

    assert_eq!(lens.center(), None);
    assert_eq!(
        lens.update(Point::new(100, 100), wide_screen()),
        Some(Point::new(100, 100))
    );
    assert_eq!(lens.center(), Some(Point::new(100, 100)));
}

#[test]
fn the_lens_leads_in_the_direction_of_travel() {
    let mut lens = FocusLens::new(2.0, 10, 0, MotionMode::Full);
    lens.update(Point::new(100, 100), wide_screen());

    // Moving right leads the target right; the centre moves halfway to it.
    assert_eq!(
        lens.update(Point::new(140, 100), wide_screen()),
        Some(Point::new(125, 100))
    );
}

#[test]
fn a_move_inside_the_dead_zone_does_not_move_the_lens() {
    let mut lens = FocusLens::new(2.0, 10, 8, MotionMode::Full);
    lens.update(Point::new(100, 100), wide_screen());

    assert_eq!(
        lens.update(Point::new(103, 100), wide_screen()),
        Some(Point::new(100, 100)),
        "a trembling pointer must not shake the lens"
    );

    // A move past the dead zone does move it.
    assert_eq!(
        lens.update(Point::new(120, 100), wide_screen()),
        Some(Point::new(115, 100))
    );
}

#[test]
fn the_lens_moves_smoothly_not_by_jumps() {
    let mut lens = FocusLens::new(2.0, 0, 0, MotionMode::Full);
    lens.update(Point::new(100, 100), wide_screen());

    // Halfway, not all the way: the lens eases instead of jumping.
    assert_eq!(
        lens.update(Point::new(200, 100), wide_screen()),
        Some(Point::new(150, 100))
    );
}

#[test]
fn reduced_motion_hides_the_lens() {
    let mut lens = FocusLens::new(2.0, 10, 0, MotionMode::Reduced);

    assert_eq!(lens.update(Point::new(100, 100), screen()), None);
    assert_eq!(lens.center(), None);

    lens.set_motion(MotionMode::Full);
    assert_eq!(
        lens.update(Point::new(100, 100), screen()),
        Some(Point::new(100, 100))
    );
}

#[test]
fn the_source_rect_shrinks_with_magnification() {
    let mut lens = FocusLens::new(2.0, 0, 0, MotionMode::Full);
    lens.update(Point::new(50, 50), screen());
    assert_eq!(lens.source_rect(screen()), Some(Rect::new(25, 25, 50, 50)));

    lens.set_magnification(4.0);
    assert_eq!(lens.source_rect(screen()), Some(Rect::new(38, 38, 25, 25)));
}

#[test]
fn the_source_rect_clamps_to_the_screen() {
    let mut lens = FocusLens::new(1.0, 0, 0, MotionMode::Full);
    lens.update(Point::new(5, 5), screen());

    assert_eq!(lens.source_rect(screen()), Some(Rect::new(0, 0, 100, 100)));
}

#[test]
fn the_source_rect_is_none_before_the_lens_is_placed() {
    let lens = FocusLens::new(2.0, 0, 0, MotionMode::Full);
    assert_eq!(lens.source_rect(screen()), None);
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/FOCUS.md");
    for word in ["ruler", "lens", "reduced", "jitter"] {
        assert!(doc.contains(word), "docs/FOCUS.md does not state `{word}`");
    }
}
