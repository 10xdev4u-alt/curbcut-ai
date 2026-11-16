//! Tests for stop-motion mode.

use curbcut_core::id::StableId;
use curbcut_core::types::Rect;

use super::*;

fn motion(id: u64, kind: MotionKind, x: i32, y: i32) -> MotionRegion {
    MotionRegion::new(StableId(id), Rect::new(x, y, 100, 50), kind)
}

#[test]
fn motion_kinds_are_named_and_resolvable() {
    assert_eq!(MotionKind::ALL.len(), 3);
    assert_eq!(MotionKind::Carousel.as_str(), "carousel");
    assert_eq!(MotionKind::Autoplay.as_str(), "autoplay");
    assert_eq!(MotionKind::Countdown.as_str(), "countdown");
    assert_eq!(MotionKind::Countdown.label(), "countdown timer");
    for kind in MotionKind::ALL {
        assert_eq!(MotionKind::from_name(kind.as_str()), Some(kind));
    }
    assert_eq!(MotionKind::from_name("Carousel"), None);
    assert_eq!(MotionKind::from_name("spinner"), None);
    assert_eq!(MotionKind::from_name(""), None);
}

#[test]
fn a_motion_region_exposes_its_fields() {
    let region = motion(7, MotionKind::Carousel, 10, 20);
    assert_eq!(region.stable_id(), StableId(7));
    assert_eq!(region.kind(), MotionKind::Carousel);
    assert_eq!(region.rect(), Rect::new(10, 20, 100, 50));
}

#[test]
fn the_config_defaults_freeze_everything() {
    let config = StopMotionConfig::default();
    assert!(config.freeze_carousels);
    assert!(config.pause_autoplay);
    assert!(config.hide_countdowns);
    assert!(config.show_play_button);
    assert!(config.is_all_frozen());
    assert!(!config.is_empty());
    assert_eq!(
        config.frozen_kinds(),
        vec![
            MotionKind::Carousel,
            MotionKind::Autoplay,
            MotionKind::Countdown
        ]
    );
}

#[test]
fn the_config_switches_kinds_independently() {
    let mut config = StopMotionConfig::new(true, false, false, true);
    assert_eq!(config.frozen_kinds(), vec![MotionKind::Carousel]);
    assert!(config.freezes(MotionKind::Carousel));
    assert!(!config.freezes(MotionKind::Autoplay));
    assert!(!config.freezes(MotionKind::Countdown));

    config.set_freeze(MotionKind::Autoplay, true);
    assert_eq!(
        config.frozen_kinds(),
        vec![MotionKind::Carousel, MotionKind::Autoplay]
    );

    config.set_freeze(MotionKind::Carousel, false);
    config.set_freeze(MotionKind::Autoplay, false);
    assert!(config.is_empty());
    assert!(!config.is_all_frozen());

    config.set_all(true);
    assert!(config.is_all_frozen());
    config.set_all(false);
    assert!(config.is_empty());
}

#[test]
fn a_frozen_region_starts_covered() {
    let region = FrozenRegion::new(
        StableId(1),
        Rect::new(0, 0, 10, 10),
        MotionKind::Autoplay,
        42,
        true,
    );
    assert_eq!(region.stable_id(), StableId(1));
    assert_eq!(region.kind(), MotionKind::Autoplay);
    assert_eq!(region.snapshot_hash(), 42);
    assert!(region.has_play_button());
    assert!(region.is_covered());
    assert!(!region.is_restored());

    let restored = region.restored();
    assert!(restored.is_restored());
    assert!(!restored.is_covered());
    assert_eq!(restored.snapshot_hash(), 42, "the hash is unchanged");
    assert!(region.is_covered(), "the original is unchanged");
}

#[test]
fn an_empty_plan_covers_nothing() {
    let plan = StopMotionPlan::new();
    assert!(plan.is_empty());
    assert_eq!(plan.len(), 0);
    assert_eq!(plan.covered_count(), 0);
    assert_eq!(plan.restored_count(), 0);
    assert!(plan.kinds().is_empty());
    assert!(plan.regions().is_empty());
    assert!(!plan.should_cover(Rect::new(0, 0, 10, 10)));
    assert!(!plan.is_covered(StableId(1)));
    assert!(plan.get(StableId(1)).is_none());
}

#[test]
fn the_planner_filters_by_config() {
    let motions = vec![
        motion(1, MotionKind::Carousel, 0, 0),
        motion(2, MotionKind::Autoplay, 200, 0),
        motion(3, MotionKind::Countdown, 400, 0),
    ];

    let planner = StopMotionPlanner::new();
    let plan = planner.plan(StopMotionConfig::default(), &motions);
    assert_eq!(plan.len(), 3);
    assert_eq!(
        plan.kinds(),
        vec![
            MotionKind::Carousel,
            MotionKind::Autoplay,
            MotionKind::Countdown
        ]
    );

    let mut config = StopMotionConfig::default();
    config.set_freeze(MotionKind::Autoplay, false);
    config.set_freeze(MotionKind::Countdown, false);
    let plan = planner.plan(config, &motions);
    assert_eq!(plan.len(), 1);
    assert_eq!(plan.kinds(), vec![MotionKind::Carousel]);
    assert_eq!(
        plan.get(StableId(1)).map(FrozenRegion::kind),
        Some(MotionKind::Carousel)
    );

    let config = StopMotionConfig::new(false, false, false, true);
    assert!(planner.plan(config, &motions).is_empty());
}

#[test]
fn the_planner_orders_by_stable_id() {
    let motions = vec![
        motion(9, MotionKind::Carousel, 0, 0),
        motion(2, MotionKind::Carousel, 200, 0),
        motion(5, MotionKind::Carousel, 400, 0),
    ];
    let plan = StopMotionPlanner::new().plan(StopMotionConfig::default(), &motions);
    let ids: Vec<u64> = plan
        .regions()
        .iter()
        .map(|region| region.stable_id().get())
        .collect();
    assert_eq!(ids, vec![2, 5, 9]);
}

#[test]
fn the_planner_hashes_the_snapshot_from_identity_and_geometry() {
    let first = motion(1, MotionKind::Carousel, 0, 0);
    let same = motion(1, MotionKind::Carousel, 0, 0);
    let other_rect = motion(1, MotionKind::Carousel, 10, 0);
    let other_id = motion(2, MotionKind::Carousel, 0, 0);
    let other_kind = motion(1, MotionKind::Autoplay, 0, 0);

    let planner = StopMotionPlanner::new();
    let plan = planner.plan(StopMotionConfig::default(), &[first.clone(), same.clone()]);
    assert_eq!(plan.len(), 2, "the input is not deduplicated");
    assert_eq!(
        plan.regions()[0].snapshot_hash(),
        plan.regions()[1].snapshot_hash(),
        "the same region hashes the same"
    );

    let hash = plan.regions()[0].snapshot_hash();
    assert_ne!(
        hash,
        planner
            .plan(StopMotionConfig::default(), &[other_rect])
            .regions()[0]
            .snapshot_hash()
    );
    assert_ne!(
        hash,
        planner
            .plan(StopMotionConfig::default(), &[other_id])
            .regions()[0]
            .snapshot_hash()
    );
    assert_ne!(
        hash,
        planner
            .plan(StopMotionConfig::default(), &[other_kind])
            .regions()[0]
            .snapshot_hash()
    );
}

#[test]
fn the_planner_omits_the_play_button_when_disabled() {
    let motions = vec![motion(1, MotionKind::Carousel, 0, 0)];
    let config = StopMotionConfig::new(true, true, true, false);

    let plan = StopMotionPlanner::new().plan(config, &motions);
    assert!(!plan.regions()[0].has_play_button());

    let plan = StopMotionPlanner::new().plan(StopMotionConfig::default(), &motions);
    assert!(plan.regions()[0].has_play_button());
}

#[test]
fn should_cover_uses_geometry() {
    let motions = vec![motion(1, MotionKind::Carousel, 100, 100)];
    let plan = StopMotionPlanner::new().plan(StopMotionConfig::default(), &motions);

    assert!(
        plan.should_cover(Rect::new(100, 100, 10, 10)),
        "an exact hit"
    );
    assert!(
        plan.should_cover(Rect::new(150, 120, 200, 200)),
        "an overlap"
    );
    assert!(!plan.should_cover(Rect::new(0, 0, 50, 50)), "far away");
    assert!(
        !plan.should_cover(Rect::new(200, 100, 10, 10)),
        "touching edges do not overlap"
    );
}

#[test]
fn restore_lifts_one_region_only() {
    let motions = vec![
        motion(1, MotionKind::Carousel, 0, 0),
        motion(2, MotionKind::Autoplay, 200, 0),
    ];
    let mut plan = StopMotionPlanner::new().plan(StopMotionConfig::default(), &motions);

    assert!(plan.restore(StableId(1)));
    assert_eq!(plan.covered_count(), 1);
    assert_eq!(plan.restored_count(), 1);
    assert!(plan.is_covered(StableId(2)));
    assert!(!plan.is_covered(StableId(1)));
    assert!(!plan.should_cover(Rect::new(0, 0, 10, 10)));
    assert!(plan.should_cover(Rect::new(200, 0, 10, 10)));
}

#[test]
fn restore_is_false_for_an_unknown_or_restored_id() {
    let motions = vec![motion(1, MotionKind::Carousel, 0, 0)];
    let mut plan = StopMotionPlanner::new().plan(StopMotionConfig::default(), &motions);

    assert!(!plan.restore(StableId(99)), "an unknown id");
    assert!(plan.restore(StableId(1)));
    assert!(!plan.restore(StableId(1)), "an already restored id");
}

#[test]
fn restore_all_counts_only_covered() {
    let motions = vec![
        motion(1, MotionKind::Carousel, 0, 0),
        motion(2, MotionKind::Autoplay, 200, 0),
        motion(3, MotionKind::Countdown, 400, 0),
    ];
    let mut plan = StopMotionPlanner::new().plan(StopMotionConfig::default(), &motions);
    plan.restore(StableId(2));

    assert_eq!(plan.restore_all(), 2, "only the covered ones");
    assert_eq!(plan.covered_count(), 0);
    assert_eq!(plan.restored_count(), 3);
    assert!(!plan.should_cover(Rect::new(0, 0, 10, 10)));
}

#[test]
fn a_session_starts_disabled_and_covers_nothing() {
    let session = StopMotionSession::new(StopMotionConfig::default());
    assert!(!session.is_enabled());
    assert!(session.plan().is_empty());
    assert!(!session.should_cover(Rect::new(0, 0, 10, 10)));
}

#[test]
fn applying_motion_populates_the_plan() {
    let mut session = StopMotionSession::new(StopMotionConfig::default());
    let motions = vec![motion(1, MotionKind::Carousel, 0, 0)];
    assert_eq!(session.apply(&motions), 1);
    assert!(!session.is_enabled(), "applying does not enable the mode");
    assert!(!session.should_cover(Rect::new(0, 0, 10, 10)));

    session.enable();
    assert!(session.is_enabled());
    assert!(session.should_cover(Rect::new(0, 0, 10, 10)));
    assert_eq!(session.plan().len(), 1);
}

#[test]
fn disabling_lifts_every_snapshot() {
    let mut session = StopMotionSession::enabled(StopMotionConfig::default());
    session.apply(&[
        motion(1, MotionKind::Carousel, 0, 0),
        motion(2, MotionKind::Autoplay, 200, 0),
    ]);
    assert_eq!(session.plan().covered_count(), 2);

    session.disable();
    assert!(!session.is_enabled());
    assert_eq!(session.plan().covered_count(), 0);
    assert_eq!(session.plan().restored_count(), 2);
    assert!(!session.should_cover(Rect::new(0, 0, 10, 10)));
}

#[test]
fn toggling_flips_the_flag() {
    let mut session = StopMotionSession::new(StopMotionConfig::default());
    assert!(session.toggle());
    assert!(session.is_enabled());
    assert!(!session.toggle());
    assert!(!session.is_enabled());
}

#[test]
fn set_config_changes_the_plan() {
    let mut session = StopMotionSession::new(StopMotionConfig::default());
    let motions = vec![
        motion(1, MotionKind::Carousel, 0, 0),
        motion(2, MotionKind::Autoplay, 200, 0),
    ];
    session.apply(&motions);
    assert_eq!(session.plan().len(), 2);

    let mut config = StopMotionConfig::default();
    config.set_freeze(MotionKind::Autoplay, false);
    session.set_config(config);
    assert_eq!(
        session.config().frozen_kinds(),
        vec![MotionKind::Carousel, MotionKind::Countdown]
    );

    session.apply(&motions);
    assert_eq!(session.plan().len(), 1);
}

#[test]
fn the_session_restores_through_the_plan() {
    let mut session = StopMotionSession::enabled(StopMotionConfig::default());
    session.apply(&[motion(1, MotionKind::Carousel, 0, 0)]);
    assert!(session.restore(StableId(1)));
    assert!(!session.should_cover(Rect::new(0, 0, 10, 10)));
    assert_eq!(session.restore_all(), 0, "already restored");
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/STOP_MOTION.md");
    for word in [
        "stop-motion",
        "carousel",
        "autoplay",
        "countdown",
        "play button",
    ] {
        assert!(
            doc.contains(word),
            "docs/STOP_MOTION.md does not state `{word}`"
        );
    }
}
