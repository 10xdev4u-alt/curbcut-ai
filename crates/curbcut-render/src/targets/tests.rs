//! Tests for large-target mode.

use super::*;

fn target(id: u64, x: i32, y: i32, width: u32, height: u32) -> Target {
    Target::new(TargetId(id), Rect::new(x, y, width, height))
}

#[test]
fn the_minimum_is_60_and_the_default_spacing_is_8() {
    assert_eq!(MIN_TARGET_PX, 60);
    assert_eq!(MIN_SPACING_PX, 8);

    let layout = TargetLayout::default();
    assert_eq!(layout.min_size_px(), 60);
    assert_eq!(layout.spacing_px(), 8);
}

#[test]
fn a_requested_minimum_is_clamped_up_to_60() {
    assert_eq!(TargetLayout::new(40, 8).min_size_px(), 60);
    assert_eq!(TargetLayout::new(0, 8).min_size_px(), 60);
    assert_eq!(TargetLayout::new(100, 8).min_size_px(), 100);
    assert_eq!(TargetLayout::new(60, 12).spacing_px(), 12);
}

#[test]
fn target_ids_expose_their_value() {
    assert_eq!(TargetId(7).get(), 7);
    assert_eq!(TargetId(0).get(), 0);
    assert_eq!(TargetId(7).to_string(), "TargetId(7)");
    assert!(TargetId(1) < TargetId(2));
}

#[test]
fn a_small_target_is_scaled_up_to_the_minimum() {
    let layout = TargetLayout::default();
    let fitted = layout.fit(target(1, 100, 100, 30, 20), Rect::new(0, 0, 1000, 1000));

    assert_eq!(fitted.id(), TargetId(1));
    assert_eq!(fitted.rect(), Rect::new(70, 80, 90, 60));
    assert_eq!(fitted.scale(), 3.0);
    assert!(!fitted.is_clipped());
    assert!(fitted.meets_minimum(60));
}

#[test]
fn a_large_target_is_never_shrunk() {
    let layout = TargetLayout::default();
    let fitted = layout.fit(target(1, 100, 100, 100, 80), Rect::new(0, 0, 1000, 1000));

    assert_eq!(fitted.rect(), Rect::new(100, 100, 100, 80));
    assert_eq!(fitted.scale(), 1.0);
    assert!(!fitted.is_clipped());
}

#[test]
fn a_scaled_target_is_translated_to_stay_inside_the_bounds() {
    let layout = TargetLayout::default();
    let fitted = layout.fit(target(1, 190, 190, 10, 10), Rect::new(0, 0, 200, 200));

    assert_eq!(fitted.rect(), Rect::new(140, 140, 60, 60));
    assert_eq!(fitted.scale(), 6.0);
    assert!(!fitted.is_clipped(), "it fits, so it is not clipped");
    assert!(fitted.meets_minimum(60));
}

#[test]
fn a_target_that_cannot_meet_the_minimum_is_clamped_and_marked_clipped() {
    let layout = TargetLayout::default();
    let fitted = layout.fit(target(1, 0, 0, 20, 20), Rect::new(0, 0, 50, 50));

    assert_eq!(fitted.rect(), Rect::new(0, 0, 50, 50));
    assert_eq!(fitted.scale(), 3.0);
    assert!(fitted.is_clipped());
    assert!(!fitted.meets_minimum(60));
}

#[test]
fn an_empty_target_is_clipped() {
    let layout = TargetLayout::default();
    let fitted = layout.fit(target(1, 10, 10, 0, 10), Rect::new(0, 0, 200, 200));

    assert_eq!(fitted.rect(), Rect::new(10, 0, 0, 60));
    assert!(fitted.is_clipped());
    assert!(!fitted.meets_minimum(60));
}

#[test]
fn spacing_is_reported_per_axis() {
    let a = Rect::new(0, 0, 60, 60);
    assert!(TargetLayout::are_spaced(a, Rect::new(68, 0, 60, 60), 8));
    assert!(TargetLayout::are_spaced(a, Rect::new(0, 68, 60, 60), 8));
    assert!(!TargetLayout::are_spaced(a, Rect::new(67, 0, 60, 60), 8));
    assert!(!TargetLayout::are_spaced(a, Rect::new(0, 0, 60, 60), 8));
    assert!(TargetLayout::are_spaced(a, Rect::new(1000, 0, 10, 10), 8));
}

#[test]
fn overlapping_targets_are_pushed_apart() {
    let layout = TargetLayout::default();
    let bounds = Rect::new(0, 0, 1000, 1000);
    let fitted = layout.layout(&[target(1, 0, 0, 60, 60), target(2, 30, 0, 60, 60)], bounds);

    assert_eq!(fitted[0].rect(), Rect::new(0, 0, 60, 60));
    assert_eq!(fitted[1].rect(), Rect::new(68, 0, 60, 60));
    assert!(!fitted[1].is_clipped());
    assert!(TargetLayout::are_spaced(
        fitted[0].rect(),
        fitted[1].rect(),
        8
    ));
}

#[test]
fn a_target_to_the_left_is_pushed_left() {
    let layout = TargetLayout::default();
    let bounds = Rect::new(0, 0, 1000, 1000);
    let fitted = layout.layout(
        &[target(1, 500, 500, 60, 60), target(2, 440, 500, 60, 60)],
        bounds,
    );

    assert_eq!(fitted[1].rect(), Rect::new(432, 500, 60, 60));
    assert!(
        !fitted[1].is_clipped(),
        "there is room, so it must not be flagged"
    );
    assert!(TargetLayout::are_spaced(
        fitted[0].rect(),
        fitted[1].rect(),
        8
    ));
}

#[test]
fn a_target_above_is_pushed_up() {
    let layout = TargetLayout::default();
    let bounds = Rect::new(0, 0, 1000, 1000);
    let fitted = layout.layout(
        &[target(1, 500, 500, 60, 60), target(2, 500, 440, 60, 60)],
        bounds,
    );

    assert_eq!(fitted[1].rect(), Rect::new(500, 432, 60, 60));
    assert!(!fitted[1].is_clipped());
    assert!(TargetLayout::are_spaced(
        fitted[0].rect(),
        fitted[1].rect(),
        8
    ));
}

#[test]
fn a_three_target_layout_flags_an_infeasible_middle_target() {
    let layout = TargetLayout::default();
    let bounds = Rect::new(0, 0, 1000, 1000);
    let fitted = layout.layout(
        &[
            target(1, 234, 0, 60, 60),
            target(2, 100, 0, 60, 60),
            target(3, 164, 0, 60, 60),
        ],
        bounds,
    );

    assert!(
        fitted[2].is_clipped(),
        "the middle target cannot satisfy both neighbours"
    );
}

#[test]
fn every_pair_in_a_layout_is_spaced_or_flagged() {
    let layout = TargetLayout::default();
    let bounds = Rect::new(0, 0, 1000, 1000);
    let fitted = layout.layout(
        &[
            target(1, 234, 0, 60, 60),
            target(2, 100, 0, 60, 60),
            target(3, 164, 0, 60, 60),
        ],
        bounds,
    );

    for i in 0..fitted.len() {
        for j in (i + 1)..fitted.len() {
            if !TargetLayout::are_spaced(fitted[i].rect(), fitted[j].rect(), 8) {
                assert!(
                    fitted[i].is_clipped() || fitted[j].is_clipped(),
                    "an unspaced pair must be reported"
                );
            }
        }
    }
}

#[test]
fn a_custom_profile_scales_and_spaces_differently() {
    let layout = TargetLayout::new(100, 12);
    let bounds = Rect::new(0, 0, 1000, 1000);
    let fitted = layout.layout(&[target(1, 0, 0, 40, 40), target(2, 50, 0, 40, 40)], bounds);

    assert_eq!(fitted[0].rect(), Rect::new(0, 0, 100, 100));
    assert_eq!(fitted[1].rect(), Rect::new(112, 0, 100, 100));
    assert!(TargetLayout::are_spaced(
        fitted[0].rect(),
        fitted[1].rect(),
        12
    ));
    assert!(!TargetLayout::are_spaced(
        fitted[0].rect(),
        fitted[1].rect(),
        13
    ));
}

#[test]
fn targets_that_cannot_be_separated_are_clipped() {
    let layout = TargetLayout::default();
    let bounds = Rect::new(0, 0, 100, 100);
    let fitted = layout.layout(&[target(1, 0, 0, 60, 60), target(2, 30, 0, 60, 60)], bounds);

    assert_eq!(
        fitted[1].rect(),
        Rect::new(40, 0, 60, 60),
        "clamped to the bounds"
    );
    assert!(fitted[1].is_clipped(), "it could not be moved far enough");
}

#[test]
fn layout_preserves_input_order() {
    let layout = TargetLayout::default();
    let bounds = Rect::new(0, 0, 1000, 1000);
    let fitted = layout.layout(
        &[target(2, 0, 0, 20, 20), target(1, 200, 200, 20, 20)],
        bounds,
    );

    let ids: Vec<u64> = fitted.iter().map(|f| f.id().get()).collect();
    assert_eq!(ids, vec![2, 1]);
}

#[test]
fn layout_scales_every_target_to_the_minimum() {
    let layout = TargetLayout::default();
    let bounds = Rect::new(0, 0, 1000, 1000);
    let fitted = layout.layout(
        &[
            target(1, 0, 0, 20, 20),
            target(2, 200, 200, 30, 20),
            target(3, 500, 0, 60, 60),
        ],
        bounds,
    );

    assert_eq!(fitted.len(), 3);
    for f in &fitted {
        assert!(f.meets_minimum(60), "{} was not scaled", f.id());
        assert!(!f.is_clipped(), "{} was clipped", f.id());
    }
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/LARGE_TARGETS.md");
    for word in ["large-target", "60", "spacing", "clipped", "scale"] {
        assert!(
            doc.contains(word),
            "docs/LARGE_TARGETS.md does not state `{word}`"
        );
    }
}
