//! Tests for stability tracking and scroll compensation.

use curbcut_core::id::StableId;
use curbcut_core::types::{Point, Rect};

use super::*;

fn obs(label: &str, x: i32, y: i32, w: u32, h: u32) -> Observation {
    Observation::new(Rect::new(x, y, w, h), label_hash(label), 0.9)
}

#[test]
fn the_config_defaults_are_sane() {
    let config = StabilityConfig::default();
    assert_eq!(config.min_similarity(), 0.3);
    assert_eq!(config.max_gap_frames(), 30);
    assert!(config.scroll_compensation());
    assert_eq!(config.max_scroll_jump(), 400.0);
}

#[test]
fn the_config_is_clamped_and_mutable() {
    let mut config = StabilityConfig::new(2.0, 5, false, -10.0);
    assert_eq!(config.min_similarity(), 1.0);
    assert_eq!(config.max_gap_frames(), 5);
    assert!(!config.scroll_compensation());
    assert_eq!(config.max_scroll_jump(), 0.0);

    config.set_min_similarity(-1.0);
    assert_eq!(config.min_similarity(), 0.0);
    config.set_max_gap_frames(9);
    assert_eq!(config.max_gap_frames(), 9);
    config.set_scroll_compensation(true);
    assert!(config.scroll_compensation());
    config.set_max_scroll_jump(25.0);
    assert_eq!(config.max_scroll_jump(), 25.0);
}

#[test]
fn label_hashes_are_stable_and_distinct() {
    assert_eq!(label_hash("Save"), label_hash("Save"));
    assert_eq!(label_hash(""), label_hash(""));
    assert_ne!(label_hash("Save"), label_hash("Cancel"));
    assert_ne!(label_hash("Save"), label_hash("Save "));
}

#[test]
fn aligned_by_shifts_a_rect() {
    let observation = obs("row", 10, 20, 30, 40);
    let aligned = observation.aligned_by(Point::new(5, -7));

    assert_eq!(aligned.rect(), Rect::new(5, 27, 30, 40));
    assert_eq!(aligned.label_hash(), observation.label_hash());
    assert_eq!(aligned.confidence(), observation.confidence());
    assert_eq!(
        observation.rect(),
        Rect::new(10, 20, 30, 40),
        "the original is unchanged"
    );
}

#[test]
fn aligned_by_saturates_at_zero() {
    let observation = obs("row", 10, 20, 30, 40);
    let aligned = observation.aligned_by(Point::new(-100, -100));
    assert_eq!(aligned.rect(), Rect::new(110, 120, 30, 40));
}

#[test]
fn similarity_combines_overlap_and_label() {
    let a = obs("x", 0, 0, 100, 100);

    let same = obs("x", 0, 0, 100, 100);
    assert!((similarity(&a, &same) - 1.0).abs() < 1e-6);

    let other_label = obs("y", 0, 0, 100, 100);
    assert!((similarity(&a, &other_label) - 0.75).abs() < 1e-6);

    let same_label_far = obs("x", 500, 500, 100, 100);
    assert!((similarity(&a, &same_label_far) - 0.25).abs() < 1e-6);

    let other_label_far = obs("y", 500, 500, 100, 100);
    assert!((similarity(&a, &other_label_far) - 0.0).abs() < 1e-6);
}

#[test]
fn the_weights_sum_to_one() {
    assert!((IOU_WEIGHT + LABEL_WEIGHT - 1.0).abs() < 1e-6);
}

#[test]
fn an_empty_tracker_has_no_tracks() {
    let tracker = StabilityTracker::default();
    assert!(tracker.is_empty());
    assert_eq!(tracker.track_count(), 0);
    assert_eq!(tracker.last_scroll_delta(), Point::default());
    assert_eq!(tracker.scroll_delta_magnitude(), 0.0);
    assert_eq!(tracker.last_frame(), 0);
    assert!(tracker.tracks().is_empty());
}

#[test]
fn an_observation_gets_a_fresh_id_and_is_new() {
    let mut tracker = StabilityTracker::default();
    let tracks = tracker.update(0, &[obs("a", 0, 0, 100, 50)]);

    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].stable_id(), StableId(1));
    assert!(tracks[0].is_new());
    assert_eq!(tracks[0].rect(), Rect::new(0, 0, 100, 50));
    assert_eq!(tracks[0].label_hash(), label_hash("a"));
    assert_eq!(tracks[0].last_frame(), 0);
    assert_eq!(tracker.track_count(), 1);
    assert_eq!(tracker.last_frame(), 0);
}

#[test]
fn an_id_is_stable_across_frames() {
    let mut tracker = StabilityTracker::default();
    let first = tracker.update(0, &[obs("a", 0, 0, 100, 50)]);
    let id = first[0].stable_id();

    let second = tracker.update(1, &[obs("a", 0, 0, 100, 50)]);
    assert_eq!(second[0].stable_id(), id);
    assert!(!second[0].is_new(), "a matched track is not new");
    assert_eq!(second[0].last_frame(), 1);

    let third = tracker.update(2, &[obs("a", 0, 0, 100, 50)]);
    assert_eq!(third[0].stable_id(), id);
}

#[test]
fn a_moved_observation_keeps_its_id() {
    let mut tracker = StabilityTracker::default();
    let first = tracker.update(0, &[obs("a", 0, 0, 100, 50)]);
    let id = first[0].stable_id();

    let second = tracker.update(1, &[obs("a", 20, 5, 100, 50)]);
    assert_eq!(second[0].stable_id(), id);
    assert_eq!(second[0].rect(), Rect::new(20, 5, 100, 50));
}

#[test]
fn a_far_observation_becomes_a_new_track() {
    let mut tracker = StabilityTracker::default();
    let first = tracker.update(0, &[obs("a", 0, 0, 100, 50)]);
    let old_id = first[0].stable_id();

    let second = tracker.update(1, &[obs("b", 800, 800, 100, 50)]);
    assert_eq!(second.len(), 1);
    assert!(second[0].is_new());
    assert_ne!(second[0].stable_id(), old_id);
    assert_eq!(tracker.track_count(), 2, "the old track is still live");
    assert!(tracker.track(old_id).is_some());
}

#[test]
fn a_missing_observation_does_not_shift_ids() {
    let mut tracker = StabilityTracker::default();
    let first = tracker.update(
        0,
        &[
            obs("a", 0, 0, 100, 100),
            obs("b", 200, 0, 100, 100),
            obs("c", 400, 0, 100, 100),
        ],
    );
    let ids: Vec<StableId> = first.iter().map(TrackedElement::stable_id).collect();

    let second = tracker.update(1, &[obs("a", 0, 0, 100, 100), obs("c", 400, 0, 100, 100)]);
    let second_ids: Vec<StableId> = second.iter().map(TrackedElement::stable_id).collect();

    assert_eq!(second_ids, vec![ids[0], ids[2]]);
    assert_eq!(tracker.track_count(), 3, "the missing track is still live");
    assert!(tracker.track(ids[1]).is_some());
}

#[test]
fn new_ids_follow_the_id_order() {
    let mut tracker = StabilityTracker::default();
    let first = tracker.update(0, &[obs("a", 0, 0, 10, 10)]);
    let second = tracker.update(1, &[obs("b", 100, 0, 10, 10)]);

    assert_eq!(first[0].stable_id(), StableId(1));
    assert_eq!(second[0].stable_id(), StableId(2));
}

#[test]
fn ids_are_stable_across_a_hundred_scrolling_frames() {
    let mut tracker = StabilityTracker::default();
    let mut first_ids: Vec<StableId> = Vec::new();

    for frame in 0..100_u64 {
        let y_offset = 10 * frame as i32;
        let observations: Vec<Observation> = (0..5)
            .map(|index| obs(&format!("item-{index}"), 0, 40 * index - y_offset, 200, 30))
            .collect();

        let tracks = tracker.update(frame, &observations);
        assert_eq!(tracks.len(), 5);

        if frame == 0 {
            first_ids = tracks.iter().map(TrackedElement::stable_id).collect();
            assert_eq!(
                first_ids,
                vec![
                    StableId(1),
                    StableId(2),
                    StableId(3),
                    StableId(4),
                    StableId(5)
                ]
            );
        } else {
            let ids: Vec<StableId> = tracks.iter().map(TrackedElement::stable_id).collect();
            assert_eq!(ids, first_ids, "an id changed at frame {frame}");
            assert_eq!(tracker.last_scroll_delta(), Point::new(0, -10));
        }
    }

    assert_eq!(tracker.track_count(), 5);
    assert_eq!(tracker.tracks().len(), 5);
}

#[test]
fn scroll_compensation_aligns_rects_within_two_pixels() {
    let mut tracker = StabilityTracker::default();
    tracker.update(
        0,
        &[
            obs("row-a", 100, 200, 80, 40),
            obs("row-b", 100, 300, 80, 40),
        ],
    );
    let before = tracker.tracks()[0].rect();

    let scrolled_a = obs("row-a", 100, 190, 80, 40);
    let scrolled_b = obs("row-b", 100, 290, 80, 40);
    let tracks = tracker.update(1, &[scrolled_a.clone(), scrolled_b]);
    let delta = tracker.last_scroll_delta();

    assert_eq!(delta, Point::new(0, -10));
    assert_eq!(
        tracks[0].rect(),
        Rect::new(100, 190, 80, 40),
        "raw coordinates are kept"
    );

    let aligned = scrolled_a.aligned_by(delta);
    let distance = (aligned.rect().y - before.y).abs();
    assert!(distance <= 2, "the aligned rect is {distance} px off");
    assert_eq!(aligned.rect(), before);
    assert!(aligned.rect().iou(before) >= 0.99);
}

#[test]
fn a_single_pair_cannot_invent_a_scroll() {
    let mut tracker = StabilityTracker::default();
    let first = tracker.update(0, &[obs("a", 0, 0, 10, 10)]);
    let id = first[0].stable_id();

    let second = tracker.update(1, &[obs("b", 800, 800, 10, 10)]);
    assert_eq!(tracker.last_scroll_delta(), Point::default());
    assert_ne!(
        second[0].stable_id(),
        id,
        "a single far pair is a new element"
    );
}

#[test]
fn a_single_new_element_does_not_fake_a_scroll() {
    let mut tracker = StabilityTracker::default();
    tracker.update(0, &[obs("a", 0, 0, 100, 50), obs("b", 0, 1000, 100, 50)]);
    let tracks = tracker.update(1, &[obs("a", 0, 0, 100, 50), obs("c", 500, 500, 100, 50)]);

    assert_eq!(
        tracker.last_scroll_delta(),
        Point::default(),
        "no consensus, no scroll"
    );
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].rect(), Rect::new(0, 0, 100, 50));
    assert_eq!(tracks[1].rect(), Rect::new(500, 500, 100, 50));
    assert!(tracks[1].is_new());
    assert_eq!(tracker.track_count(), 3, "the unmatched track stays live");
}

#[test]
fn a_large_scroll_jump_is_clamped() {
    let config = StabilityConfig::new(0.3, 30, true, 20.0);
    let mut tracker = StabilityTracker::new(config);
    tracker.update(
        0,
        &[
            obs("a", 0, 0, 100, 50),
            obs("b", 0, 2000, 100, 50),
            obs("c", 0, 4000, 100, 50),
        ],
    );
    tracker.update(
        1,
        &[
            obs("a", 0, -300, 100, 50),
            obs("b", 0, 1700, 100, 50),
            obs("c", 0, 3700, 100, 50),
        ],
    );

    assert_eq!(tracker.last_scroll_delta(), Point::new(0, -20));
    assert!(tracker.scroll_delta_magnitude() <= 20.0 + 1e-6);
}

#[test]
fn scroll_compensation_can_be_disabled() {
    let mut config = StabilityConfig::default();
    config.set_scroll_compensation(false);
    let mut tracker = StabilityTracker::new(config);

    tracker.update(0, &[obs("x", 0, 0, 100, 50)]);
    let tracks = tracker.update(1, &[obs("x", 0, -10, 100, 50)]);

    assert_eq!(tracker.last_scroll_delta(), Point::default());
    assert_eq!(
        tracks.len(),
        1,
        "the overlap still matches without compensation"
    );
    assert_eq!(tracks[0].rect(), Rect::new(0, -10, 100, 50));
}

#[test]
fn a_track_survives_a_short_gap_and_dies_after_it() {
    let mut tracker = StabilityTracker::default();
    tracker.update(0, &[obs("a", 0, 0, 10, 10)]);
    assert_eq!(tracker.track_count(), 1);

    tracker.update(30, &[]);
    assert_eq!(tracker.track_count(), 1, "30 frames is within the gap");

    tracker.update(61, &[]);
    assert_eq!(tracker.track_count(), 0, "61 frames is past the gap");
    assert!(tracker.is_empty());
}

#[test]
fn the_gap_limit_is_configurable() {
    let config = StabilityConfig::new(0.3, 1, true, 400.0);
    let mut tracker = StabilityTracker::new(config);
    tracker.update(0, &[obs("a", 0, 0, 10, 10)]);

    tracker.update(1, &[]);
    assert_eq!(tracker.track_count(), 1);
    tracker.update(2, &[]);
    assert_eq!(tracker.track_count(), 0);
}

#[test]
fn stable_regions_skip_reanalysis() {
    let mut tracker = StabilityTracker::default();
    tracker.update(0, &[obs("save", 0, 0, 100, 50)]);

    assert!(!tracker.should_reanalyze(Rect::new(0, 0, 100, 50)));
    assert!(
        !tracker.should_reanalyze(Rect::new(50, 25, 20, 10)),
        "a covered region is stable"
    );
    assert!(
        tracker.should_reanalyze(Rect::new(500, 500, 100, 50)),
        "an uncovered region is not"
    );
}

#[test]
fn motion_forces_reanalysis() {
    let mut tracker = StabilityTracker::default();
    tracker.update(
        0,
        &[obs("save", 0, 0, 100, 50), obs("cancel", 0, 200, 100, 50)],
    );
    assert!(!tracker.should_reanalyze(Rect::new(0, 0, 100, 50)));

    tracker.update(
        1,
        &[obs("save", 0, -10, 100, 50), obs("cancel", 0, 190, 100, 50)],
    );
    assert_eq!(tracker.last_scroll_delta(), Point::new(0, -10));
    assert!(
        tracker.should_reanalyze(Rect::new(0, -10, 100, 50)),
        "motion is detected"
    );
}

#[test]
fn a_changed_label_is_a_content_change() {
    let mut tracker = StabilityTracker::default();
    let tracks = tracker.update(0, &[obs("save", 0, 0, 100, 50)]);
    let id = tracks[0].stable_id();

    assert!(!tracker.content_changed(id, label_hash("save")));
    assert!(tracker.content_changed(id, label_hash("cancel")));
    assert!(tracker.content_changed(StableId(999), label_hash("save")));
}

#[test]
fn missed_frames_counts_the_gap() {
    let track = TrackedElement::new(StableId(3), Rect::new(0, 0, 10, 10), 7, 4, 0.5);
    assert_eq!(track.missed_frames(10), 6);
    assert_eq!(track.missed_frames(4), 0, "saturating");
    assert_eq!(track.stable_id(), StableId(3));
    assert_eq!(track.label_hash(), 7);
    assert_eq!(track.confidence(), 0.5);
    assert!(track.is_new());
    assert_eq!(track.to_observation().rect(), Rect::new(0, 0, 10, 10));
}

#[test]
fn the_tracker_is_deterministic() {
    let run = || {
        let mut tracker = StabilityTracker::default();
        let mut seen = Vec::new();
        for frame in 0..20_u64 {
            let observations = vec![
                obs("a", 0, -5 * frame as i32, 100, 50),
                obs("b", 150, 20, 80, 40),
                obs("c", 400, -3 * frame as i32, 60, 30),
            ];
            seen.push(tracker.update(frame, &observations));
        }
        seen
    };

    assert_eq!(run(), run());
}

#[test]
fn reset_clears_everything() {
    let mut tracker = StabilityTracker::default();
    tracker.update(0, &[obs("a", 0, 0, 10, 10), obs("b", 50, 0, 10, 10)]);
    assert_eq!(tracker.track_count(), 2);

    tracker.reset();
    assert!(tracker.is_empty());
    assert_eq!(tracker.last_scroll_delta(), Point::default());
    assert_eq!(tracker.last_frame(), 0);

    let tracks = tracker.update(0, &[obs("a", 0, 0, 10, 10)]);
    assert_eq!(
        tracks[0].stable_id(),
        StableId(1),
        "the id counter restarts"
    );
}

#[test]
fn an_empty_frame_does_not_panic() {
    let mut tracker = StabilityTracker::default();
    assert!(tracker.update(0, &[]).is_empty());
    assert!(tracker.update(1, &[]).is_empty());
    assert_eq!(tracker.track_count(), 0);
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/STABILITY.md");
    for word in ["stable", "scroll", "hungarian", "iou", "re-analyze"] {
        assert!(
            doc.contains(word),
            "docs/STABILITY.md does not state `{word}`"
        );
    }
}
