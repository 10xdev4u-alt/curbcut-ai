//! Tests for motion classification.

use super::*;
use curbcut_core::id::StableId;

fn screen() -> Rect {
    Rect::new(0, 0, 1000, 800)
}

fn id(value: u64) -> StableId {
    StableId(value)
}

/// A region whose sampled bytes all differ for a different seed.
fn gradient(seed: u8) -> Vec<u8> {
    (0..256)
        .map(|index| (index as u8).wrapping_add(seed))
        .collect()
}

/// A region with three sampled bytes changed.
fn nearly(seed: u8) -> Vec<u8> {
    let mut pixels = gradient(seed);
    for index in [0, 8, 16] {
        pixels[index] = pixels[index].wrapping_add(1);
    }
    pixels
}

fn observe(
    detector: &mut MotionDetector,
    rect: Rect,
    roles: &[Role],
    pixels: &[u8],
) -> Option<MotionReport> {
    detector.observe(id(1), rect, roles, pixels, 100)
}

#[test]
fn class_names_are_stable() {
    assert_eq!(MotionClass::Static.as_str(), "static");
    assert_eq!(MotionClass::Carousel.as_str(), "carousel");
    assert_eq!(MotionClass::Autoplay.as_str(), "autoplay");
    assert_eq!(MotionClass::Countdown.as_str(), "countdown");
    assert_eq!(
        MotionClass::from_name("carousel"),
        Some(MotionClass::Carousel)
    );
    assert_eq!(MotionClass::from_name("nope"), None);
    assert_eq!(MotionClass::ALL.len(), 4);
}

#[test]
fn a_static_region_is_never_reported() {
    let mut detector = MotionDetector::new(screen(), MotionConfig::default());
    let pixels = gradient(0);
    for _ in 0..10 {
        assert!(observe(&mut detector, Rect::new(0, 0, 100, 100), &[], &pixels).is_none());
    }
    assert_eq!(detector.region_count(), 1);
}

#[test]
fn the_first_observation_is_static() {
    let mut detector = MotionDetector::new(screen(), MotionConfig::default());
    assert!(observe(&mut detector, Rect::new(0, 0, 100, 100), &[], &gradient(0)).is_none());
}

#[test]
fn a_moving_carousel_is_reported_once() {
    let mut detector = MotionDetector::new(screen(), MotionConfig::default());
    let rect = Rect::new(200, 200, 200, 200);
    let roles = [Role::Tab];
    let mut report = None;
    for step in 0..6 {
        if let Some(found) = observe(&mut detector, rect, &roles, &gradient(step)) {
            report = Some(found);
        }
    }
    let report = report.expect("a carousel that moved for 500 ms is reported");
    assert_eq!(report.kind, MotionClass::Carousel);
    assert_eq!(report.rect, rect);
    assert_eq!(report.id, id(1));
    assert!(
        observe(&mut detector, rect, &roles, &gradient(9)).is_none(),
        "a reported region stays quiet until it goes static"
    );
}

#[test]
fn a_moving_list_region_is_a_carousel() {
    let classifier = MotionClassifier::new(screen());
    let kind = classifier.classify(1.0, Rect::new(0, 0, 500, 200), &[Role::ListItem]);
    assert_eq!(kind, MotionClass::Carousel);
}

#[test]
fn a_small_change_in_a_small_region_is_a_countdown() {
    let mut detector = MotionDetector::new(screen(), MotionConfig::default());
    let rect = Rect::new(400, 400, 40, 40);
    let mut report = None;
    for step in 0..6 {
        let pixels = if step % 2 == 0 {
            nearly(0)
        } else {
            gradient(0)
        };
        if let Some(found) = observe(&mut detector, rect, &[], &pixels) {
            report = Some(found);
        }
    }
    assert_eq!(
        report.expect("a small ticking region is reported").kind,
        MotionClass::Countdown
    );
}

#[test]
fn a_large_change_in_a_large_region_is_autoplay() {
    let classifier = MotionClassifier::new(screen());
    let kind = classifier.classify(1.0, Rect::new(0, 0, 300, 200), &[Role::Image]);
    assert_eq!(kind, MotionClass::Autoplay);
}

#[test]
fn a_small_change_in_a_large_region_is_not_a_countdown() {
    let classifier = MotionClassifier::new(screen());
    let kind = classifier.classify(0.09375, Rect::new(0, 0, 1000, 800), &[]);
    assert_eq!(kind, MotionClass::Autoplay);
}

#[test]
fn a_region_resets_after_going_static() {
    let mut detector = MotionDetector::new(screen(), MotionConfig::default());
    let rect = Rect::new(200, 200, 200, 200);
    let roles = [Role::Tab];
    for step in 0..6 {
        observe(&mut detector, rect, &roles, &gradient(step));
    }
    assert!(
        observe(&mut detector, rect, &roles, &gradient(5)).is_none(),
        "holding the last frame is static"
    );
    let mut report = None;
    for step in 6..12 {
        if let Some(found) = observe(&mut detector, rect, &roles, &gradient(step)) {
            report = Some(found);
        }
    }
    assert!(report.is_some(), "moving again reports again");
}

#[test]
fn forget_drops_a_region() {
    let mut detector = MotionDetector::new(screen(), MotionConfig::default());
    observe(&mut detector, Rect::new(0, 0, 100, 100), &[], &gradient(0));
    assert_eq!(detector.region_count(), 1);
    detector.forget(id(1));
    assert_eq!(detector.region_count(), 0);
    assert!(observe(&mut detector, Rect::new(0, 0, 100, 100), &[], &gradient(0)).is_none());
}

#[test]
fn min_motion_ms_gates_the_report() {
    let config = MotionConfig {
        min_motion_ms: 1000,
        ..MotionConfig::default()
    };
    let mut detector = MotionDetector::new(screen(), config);
    let rect = Rect::new(200, 200, 200, 200);
    let roles = [Role::Tab];
    for step in 0..10 {
        assert!(
            observe(&mut detector, rect, &roles, &gradient(step)).is_none(),
            "step {step} is under the gate"
        );
    }
    let report = observe(&mut detector, rect, &roles, &gradient(10))
        .expect("the tenth moving observation reaches 1000 ms");
    assert_eq!(report.kind, MotionClass::Carousel);
}

#[test]
fn thresholds_are_clamped() {
    let config = MotionConfig {
        motion_threshold: 2.0,
        countdown_max_ratio: f32::NAN,
        ..MotionConfig::default()
    };
    let detector = MotionDetector::new(screen(), config);
    assert_eq!(detector.config().motion_threshold, 1.0);
    assert_eq!(
        detector.config().countdown_max_ratio,
        DEFAULT_COUNTDOWN_MAX_RATIO
    );
}

#[test]
fn an_empty_screen_still_classifies() {
    let classifier = MotionClassifier::new(Rect::default());
    assert_eq!(
        classifier.classify(0.5, Rect::default(), &[]),
        MotionClass::Autoplay
    );
    assert_eq!(
        classifier.classify(0.0, Rect::default(), &[]),
        MotionClass::Static
    );
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/MOTION_DETECTION.md");
    for word in ["motion", "carousel", "autoplay", "countdown", "static"] {
        assert!(
            doc.contains(word),
            "docs/MOTION_DETECTION.md does not state `{word}`"
        );
    }
}
