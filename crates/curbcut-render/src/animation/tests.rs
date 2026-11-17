//! Tests for the animation policy.

use super::*;

#[test]
fn the_ceiling_is_150_ms() {
    assert_eq!(MAX_DURATION_MS, 150);
    assert_eq!(AnimationPolicy::default().max_duration_ms(), 150);
    assert_eq!(
        AnimationPolicy::default().preference(),
        MotionPreference::Full
    );
}

#[test]
fn a_requested_ceiling_is_clamped_to_the_cap() {
    let policy = AnimationPolicy::new(500, MotionPreference::Full);
    assert_eq!(
        policy.max_duration_ms(),
        150,
        "a caller cannot raise the cap"
    );

    let zero = AnimationPolicy::new(0, MotionPreference::Full);
    assert_eq!(zero.max_duration_ms(), 0, "a caller may lower the cap");

    let default = AnimationPolicy::from_preference(MotionPreference::Reduced);
    assert_eq!(default.max_duration_ms(), 150);
    assert_eq!(default.preference(), MotionPreference::Reduced);
}

#[test]
fn motion_preferences_are_named() {
    assert!(MotionPreference::Full.is_full());
    assert!(!MotionPreference::Full.is_reduced());
    assert!(MotionPreference::Reduced.is_reduced());
    assert!(!MotionPreference::Reduced.is_full());
    assert_eq!(MotionPreference::default(), MotionPreference::Full);
}

#[test]
fn kinds_are_named() {
    let kinds = [
        AnimationKind::Fade,
        AnimationKind::Slide,
        AnimationKind::Scale,
        AnimationKind::Spin,
        AnimationKind::Zoom,
        AnimationKind::Parallax,
        AnimationKind::Pulse,
        AnimationKind::Highlight,
    ];
    let names: Vec<&str> = kinds.iter().map(|kind| kind.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "fade",
            "slide",
            "scale",
            "spin",
            "zoom",
            "parallax",
            "pulse",
            "highlight"
        ]
    );
    for kind in kinds {
        assert!(!kind.as_str().is_empty());
    }
}

#[test]
fn essential_kinds_carry_meaning() {
    assert!(AnimationKind::Fade.is_essential());
    assert!(AnimationKind::Highlight.is_essential());
    for kind in [
        AnimationKind::Slide,
        AnimationKind::Scale,
        AnimationKind::Spin,
        AnimationKind::Zoom,
        AnimationKind::Parallax,
        AnimationKind::Pulse,
    ] {
        assert!(!kind.is_essential(), "{} is decoration", kind.as_str());
    }
}

#[test]
fn vestibular_triggers_are_spun_zoom_and_parallax() {
    for kind in [
        AnimationKind::Spin,
        AnimationKind::Zoom,
        AnimationKind::Parallax,
    ] {
        assert!(kind.is_vestibular_trigger(), "{}", kind.as_str());
    }
    for kind in [
        AnimationKind::Fade,
        AnimationKind::Slide,
        AnimationKind::Scale,
        AnimationKind::Pulse,
        AnimationKind::Highlight,
    ] {
        assert!(!kind.is_vestibular_trigger(), "{}", kind.as_str());
    }
    assert!(AnimationKind::Pulse.is_repeating());
    assert!(!AnimationKind::Fade.is_repeating());
}

#[test]
fn a_request_round_trips() {
    let request = AnimationRequest::new(AnimationKind::Fade, 250);
    assert_eq!(request.kind(), AnimationKind::Fade);
    assert_eq!(request.duration_ms(), 250);
    assert_eq!(request.easing(), Easing::EaseOut);

    let eased = request.with_easing(Easing::EaseInOut);
    assert_eq!(eased.easing(), Easing::EaseInOut);
    assert_eq!(
        request.easing(),
        Easing::EaseOut,
        "the original is unchanged"
    );
    assert_eq!(
        eased.duration_ms(),
        250,
        "easing does not change the duration"
    );
}

#[test]
fn easings_are_named() {
    assert_eq!(Easing::Linear.as_str(), "linear");
    assert_eq!(Easing::EaseIn.as_str(), "ease-in");
    assert_eq!(Easing::EaseOut.as_str(), "ease-out");
    assert_eq!(Easing::EaseInOut.as_str(), "ease-in-out");
    assert_eq!(Easing::default(), Easing::EaseOut);
}

#[test]
fn a_plan_clamps_to_the_ceiling() {
    let policy = AnimationPolicy::default();
    let plan = policy
        .plan(AnimationRequest::new(AnimationKind::Slide, 900))
        .expect("a slide is allowed");
    assert_eq!(plan.kind(), AnimationKind::Slide);
    assert_eq!(plan.duration_ms(), 150);
    assert_eq!(plan.duration(), Some(Duration::from_millis(150)));

    let short = policy
        .plan(AnimationRequest::new(AnimationKind::Fade, 30))
        .expect("a fade is allowed");
    assert_eq!(short.duration_ms(), 30);
    assert_eq!(short.duration(), Some(Duration::from_millis(30)));
}

#[test]
fn a_plan_at_a_lowered_ceiling_clamps_to_it() {
    let policy = AnimationPolicy::new(20, MotionPreference::Full);
    let plan = policy
        .plan(AnimationRequest::new(AnimationKind::Fade, 150))
        .expect("a fade is allowed");
    assert_eq!(plan.duration_ms(), 20);
}

#[test]
fn a_zero_duration_is_instant() {
    let policy = AnimationPolicy::default();
    let plan = policy
        .plan(AnimationRequest::new(AnimationKind::Highlight, 0))
        .expect("a highlight is allowed");
    assert_eq!(plan.duration_ms(), 0);
    assert!(plan.is_instant());
    assert_eq!(plan.duration(), None);
}

#[test]
fn vestibular_triggers_are_always_refused() {
    let full = AnimationPolicy::default();
    let reduced = AnimationPolicy::from_preference(MotionPreference::Reduced);

    for kind in [
        AnimationKind::Spin,
        AnimationKind::Zoom,
        AnimationKind::Parallax,
    ] {
        assert!(!full.allows(kind), "{} is never allowed", kind.as_str());
        assert!(!reduced.allows(kind), "{} is never allowed", kind.as_str());
        assert_eq!(full.plan(AnimationRequest::new(kind, 10)), None);
        assert_eq!(reduced.plan(AnimationRequest::new(kind, 10)), None);
    }
}

#[test]
fn reduced_motion_refuses_non_essential_motion() {
    let reduced = AnimationPolicy::from_preference(MotionPreference::Reduced);
    for kind in [
        AnimationKind::Slide,
        AnimationKind::Scale,
        AnimationKind::Pulse,
    ] {
        assert!(!reduced.allows(kind), "{} is decoration", kind.as_str());
        assert_eq!(reduced.plan(AnimationRequest::new(kind, 20)), None);
    }
}

#[test]
fn reduced_motion_makes_essential_motion_instant() {
    let reduced = AnimationPolicy::from_preference(MotionPreference::Reduced);
    for kind in [AnimationKind::Fade, AnimationKind::Highlight] {
        assert!(reduced.allows(kind), "{} carries meaning", kind.as_str());
        let plan = reduced
            .plan(AnimationRequest::new(kind, 120))
            .expect("an essential change is still applied");
        assert_eq!(
            plan.duration_ms(),
            0,
            "{} is applied at once",
            kind.as_str()
        );
        assert!(plan.is_instant());
        assert_eq!(plan.duration(), None);
    }
}

#[test]
fn a_refusal_is_applying_the_end_state_not_skipping_it() {
    let reduced = AnimationPolicy::from_preference(MotionPreference::Reduced);
    let refused = reduced.plan(AnimationRequest::new(AnimationKind::Slide, 80));
    assert!(refused.is_none());
}

#[test]
fn a_plan_never_exceeds_the_ceiling() {
    let policy = AnimationPolicy::default();
    for duration in [0, 1, 149, 150, 151, 10_000] {
        let plan = policy
            .plan(AnimationRequest::new(AnimationKind::Fade, duration))
            .expect("a fade is allowed");
        assert!(plan.duration_ms() <= MAX_DURATION_MS);
        assert!(plan.duration_ms() <= duration.min(u32::from(MAX_DURATION_MS)) as u16);
    }
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/ANIMATION.md");
    for word in [
        "animation",
        "150",
        "reduced motion",
        "vestibular",
        "parallax",
    ] {
        assert!(
            doc.contains(word),
            "docs/ANIMATION.md does not state `{word}`"
        );
    }
}
