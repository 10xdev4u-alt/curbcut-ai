//! Tests for dimming.

use super::*;

fn request(screen: Rect, focused: Option<Rect>, except: &[Rect], opacity: f32) -> DimRequest {
    DimRequest {
        screen,
        focused,
        except: except.to_vec(),
        opacity,
        motion: DimMotion::Static,
    }
}

#[test]
fn a_screen_with_nothing_protected_is_one_rect() {
    let screen = Rect::new(0, 0, 100, 100);
    let plan = dim_plan(request(screen, None, &[], 0.5)).expect("valid");

    assert_eq!(plan.rects, vec![screen]);
    assert_eq!(plan.protected, Vec::new());
    assert_eq!(plan.opacity, 0.5);
    assert!(plan.is_static());
    assert!(plan.has_no_overlap());
    assert!(plan.is_inside(screen));
    assert!(plan.covers_exactly(screen));
}

#[test]
fn the_focused_control_is_left_undimmed() {
    let screen = Rect::new(0, 0, 100, 100);
    let focused = Rect::new(40, 40, 20, 20);

    let plan = dim_plan(request(screen, Some(focused), &[], 0.6)).expect("valid");

    assert_eq!(plan.protected, vec![focused]);
    assert!(!plan.rects.iter().any(|rect| intersects(*rect, focused)));
    assert!(plan.does_not_cover_any(&plan.protected));
    assert!(plan.has_no_overlap());
    assert!(plan.is_inside(screen));
    assert!(plan.covers_exactly(screen));
    // 10000 - 400, split into four bands.
    assert_eq!(plan.total_area(), 9600);
    assert_eq!(plan.len(), 4);
}

#[test]
fn except_regions_are_left_undimmed() {
    let screen = Rect::new(0, 0, 200, 100);
    let except = [Rect::new(0, 0, 20, 20), Rect::new(180, 80, 20, 20)];

    let plan = dim_plan(request(screen, None, &except, 0.75)).expect("valid");

    assert_eq!(plan.protected, except);
    assert!(plan.does_not_cover_any(&except));
    assert!(plan.covers_exactly(screen));
    assert_eq!(plan.total_area(), 20000 - 800);
}

#[test]
fn overlapping_protected_regions_are_counted_once() {
    let screen = Rect::new(0, 0, 100, 100);
    let except = [
        Rect::new(10, 10, 40, 40),
        Rect::new(30, 30, 40, 40),
        Rect::new(80, 80, 10, 10),
    ];

    let plan = dim_plan(request(screen, None, &except, 0.5)).expect("valid");

    // Union: 1600 + 1600 - 400 + 100 = 2900.
    assert_eq!(union_area(&except), 2900);
    assert_eq!(plan.total_area(), 10000 - 2900);
    assert!(plan.covers_exactly(screen));
    assert!(plan.does_not_cover_any(&except));
}

#[test]
fn a_protected_region_outside_the_screen_is_ignored() {
    let screen = Rect::new(0, 0, 100, 100);
    let except = [Rect::new(200, 200, 50, 50), Rect::new(-50, -50, 20, 20)];

    let plan = dim_plan(request(screen, None, &except, 0.5)).expect("valid");

    assert_eq!(plan.protected, Vec::new());
    assert_eq!(plan.rects, vec![screen]);
}

#[test]
fn a_partially_offscreen_region_is_clipped() {
    let screen = Rect::new(0, 0, 100, 100);
    let except = [Rect::new(-20, 10, 40, 20)];

    let plan = dim_plan(request(screen, None, &except, 0.5)).expect("valid");

    assert_eq!(plan.protected, vec![Rect::new(0, 10, 20, 20)]);
    assert!(plan.covers_exactly(screen));
}

#[test]
fn opacity_is_configurable_and_bounded() {
    let screen = Rect::new(0, 0, 10, 10);
    assert_eq!(
        dim_plan(request(screen, None, &[], 0.0))
            .expect("valid")
            .opacity,
        0.0
    );
    assert_eq!(
        dim_plan(request(screen, None, &[], 1.0))
            .expect("valid")
            .opacity,
        1.0
    );
    assert_eq!(
        dim_plan(request(screen, None, &[], 1.5)).expect_err("too strong"),
        DimError::OpacityOutOfRange(1.5)
    );
    assert_eq!(
        dim_plan(request(screen, None, &[], -0.1)).expect_err("negative"),
        DimError::OpacityOutOfRange(-0.1)
    );
    assert!(DimError::OpacityOutOfRange(2.0)
        .to_string()
        .contains("0.0..=1.0"));
}

#[test]
fn reduced_motion_is_static() {
    let screen = Rect::new(0, 0, 10, 10);
    let mut fade = request(screen, None, &[], 0.5);
    fade.motion = DimMotion::Fade { duration_ms: 150 };
    let plan = dim_plan(fade).expect("valid");

    assert!(!plan.is_static());
    assert_eq!(plan.motion, DimMotion::Fade { duration_ms: 150 });

    let plan = dim_plan(request(screen, None, &[], 0.5)).expect("valid");
    assert!(plan.is_static());
    assert_eq!(plan.motion, DimMotion::Static);
}

#[test]
fn an_empty_screen_produces_no_rectangles() {
    let screen = Rect::new(0, 0, 0, 100);
    let plan = dim_plan(request(screen, None, &[], 0.5)).expect("valid");

    assert!(plan.is_empty());
    assert!(plan.rects.is_empty());
    assert!(plan.protected.is_empty());
    assert!(plan.is_inside(screen));
    assert!(plan.covers_exactly(screen));
}

#[test]
fn extreme_rectangles_are_total() {
    let screen = Rect::new(i32::MAX, 0, 10, 10);
    let plan = dim_plan(request(screen, None, &[], 0.5)).expect("valid");
    assert!(plan.is_empty());
    assert!(plan.is_inside(screen));
    assert!(plan.covers_exactly(screen));

    let screen = Rect::new(0, 0, u32::MAX, 10);
    let plan = dim_plan(request(screen, None, &[], 0.5)).expect("valid");
    assert!(plan.is_inside(screen));
    assert!(plan.covers_exactly(screen));
    for rect in &plan.rects {
        assert!(i64::from(rect.x) + i64::from(rect.width) <= i64::from(i32::MAX));
    }

    let screen = Rect::new(0, 0, 100, 100);
    let except = [Rect::new(i32::MAX, i32::MAX, 10, 10)];
    let plan = dim_plan(request(
        screen,
        Some(Rect::new(i32::MIN, i32::MIN, 10, 10)),
        &except,
        0.5,
    ))
    .expect("valid");
    assert!(plan.is_inside(screen));
    assert!(plan.covers_exactly(screen));
}

#[test]
fn every_sample_is_covered_at_most_once_and_never_protected() {
    let screen = Rect::new(0, 0, 60, 40);
    let except = [
        Rect::new(10, 5, 15, 10),
        Rect::new(40, 20, 10, 10),
        Rect::new(5, 30, 20, 5),
    ];
    let plan =
        dim_plan(request(screen, Some(Rect::new(25, 15, 5, 5)), &except, 0.5)).expect("valid");

    for y in 0..40 {
        for x in 0..60 {
            let point = Rect::new(x, y, 1, 1);
            let covering = plan
                .rects
                .iter()
                .filter(|rect| intersects(**rect, point))
                .count();
            let protected = plan.protected.iter().any(|rect| intersects(*rect, point));
            assert!(covering <= 1, "({x},{y}) is dimmed {covering} times");
            assert!(
                !(covering >= 1 && protected),
                "({x},{y}) is both dimmed and protected"
            );
            assert!(
                protected || covering >= 1,
                "({x},{y}) is neither dimmed nor protected"
            );
        }
    }
}

#[test]
fn the_checkers_reject_broken_plans() {
    let screen = Rect::new(0, 0, 100, 100);
    let protected = vec![Rect::new(40, 40, 20, 20)];

    let overlapping = DimPlan {
        rects: vec![Rect::new(0, 0, 60, 60), Rect::new(50, 50, 60, 60)],
        opacity: 0.5,
        motion: DimMotion::Static,
        protected: protected.clone(),
    };
    assert!(!overlapping.has_no_overlap());

    let covering_protected = DimPlan {
        rects: vec![Rect::new(30, 30, 40, 40)],
        opacity: 0.5,
        motion: DimMotion::Static,
        protected: protected.clone(),
    };
    assert!(!covering_protected.does_not_cover_any(&protected));

    let outside = DimPlan {
        rects: vec![Rect::new(50, 50, 60, 60)],
        opacity: 0.5,
        motion: DimMotion::Static,
        protected: protected.clone(),
    };
    assert!(!outside.is_inside(screen));

    let short = DimPlan {
        rects: vec![Rect::new(0, 0, 50, 50)],
        opacity: 0.5,
        motion: DimMotion::Static,
        protected: protected.clone(),
    };
    assert!(!short.covers_exactly(screen));

    let valid =
        dim_plan(request(screen, Some(Rect::new(40, 40, 20, 20)), &[], 0.5)).expect("valid");
    assert!(valid.has_no_overlap());
    assert!(valid.does_not_cover_any(&valid.protected));
    assert!(valid.is_inside(screen));
    assert!(valid.covers_exactly(screen));
}

#[test]
fn union_area_counts_overlaps_once() {
    assert_eq!(union_area(&[]), 0);
    assert_eq!(union_area(&[Rect::new(0, 0, 10, 10)]), 100);
    assert_eq!(
        union_area(&[Rect::new(0, 0, 10, 10), Rect::new(5, 5, 10, 10)]),
        175
    );
    assert_eq!(
        union_area(&[Rect::new(0, 0, 10, 10), Rect::new(0, 0, 10, 10)]),
        100
    );
    assert_eq!(union_area(&[Rect::new(0, 0, 0, 10)]), 0);
}

#[test]
fn covers_exactly_does_not_overflow_on_an_extreme_screen() {
    let screen = Rect::new(i32::MIN, i32::MIN, u32::MAX, u32::MAX);
    let plan = dim_plan(request(screen, None, &[], 0.5)).expect("valid");

    // The clamped span in each axis is `u32::MAX`; the product exceeds `i64`.
    assert!(plan.covers_exactly(screen));
}

#[test]
fn union_area_does_not_overflow_on_extreme_rectangles() {
    let first = Rect::new(i32::MIN, i32::MIN, u32::MAX, u32::MAX);
    let second = Rect::new(i32::MAX, i32::MIN, u32::MAX, u32::MAX);
    let expected = u64::from(u32::MAX) * u64::from(u32::MAX);

    // The second rectangle clips to nothing; the first is `u32::MAX²`.
    assert_eq!(union_area(&[first]), expected);
    assert_eq!(union_area(&[first, second]), expected);
    assert_eq!(union_area(&[second]), 0);
}

#[test]
fn rects_are_in_reading_order() {
    let screen = Rect::new(0, 0, 100, 100);
    let plan = dim_plan(request(screen, Some(Rect::new(40, 40, 20, 20)), &[], 0.5)).expect("valid");

    let mut sorted = plan.rects.clone();
    sorted.sort_by_key(|rect| (rect.y, rect.x));
    assert_eq!(plan.rects, sorted);
    assert_eq!(plan.len(), 4);
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/DIMMING.md");
    for rule in [
        "focused",
        "exact",
        "Static",
        "except",
        "reading order",
        "clipped",
        "u128",
    ] {
        assert!(
            doc.contains(rule),
            "docs/DIMMING.md does not state `{rule}`"
        );
    }
}
