use super::*;
use curbcut_core::types::Rect;

const CANARY: &str = "CC-CANARY-TEXT-9f86d081884c7d65";

fn config() -> DirtyConfig {
    DirtyConfig {
        grid_cols: 4,
        grid_rows: 4,
        change_threshold: 12,
        min_changed_cells: 1,
        cooldown_frames: 1,
    }
}

/// A frame of `width * height` pixels, all `value`.
fn flat(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut data = Vec::with_capacity(width as usize * height as usize * RGBA_BYTES);
    for _ in 0..(width as usize * height as usize) {
        data.extend_from_slice(&[value, value, value, 255]);
    }
    data
}

/// A frame with one quadrant set to `value`.
fn quadrant(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut data = flat(width, height, 0);
    for y in 0..height as usize {
        for x in 0..width as usize {
            if x < width as usize / 2 && y < height as usize / 2 {
                let base = (y * width as usize + x) * RGBA_BYTES;
                data[base] = value;
                data[base + 1] = value;
                data[base + 2] = value;
            }
        }
    }
    data
}

#[test]
fn an_identical_frame_has_no_changes() {
    let cfg = config();
    let frame = flat(8, 8, 200);
    let report = diff(&frame, &frame, 8, 8, &cfg).expect("a report");
    assert_eq!(report.changed_fraction, 0.0);
    assert!(report.regions.is_empty());
    assert!(!report.significant);
}

#[test]
fn a_changed_quadrant_becomes_one_region() {
    let cfg = config();
    let before = flat(8, 8, 0);
    let after = quadrant(8, 8, 255);
    let report = diff(&before, &after, 8, 8, &cfg).expect("a report");
    assert_eq!(report.regions, vec![Rect::new(0, 0, 4, 4)]);
    assert_eq!(report.changed_fraction, 4.0 / 16.0);
    assert!(report.significant);
}

#[test]
fn adjacent_changed_rows_merge_into_one_region() {
    let cfg = DirtyConfig {
        grid_cols: 2,
        grid_rows: 2,
        ..config()
    };
    let mut after = flat(4, 4, 0);
    // Change the whole left column: cells (row 0, col 0) and (row 1, col 0).
    for y in 0..4 {
        let base = (y * 4) * RGBA_BYTES;
        after[base] = 255;
        after[base + 1] = 255;
        after[base + 2] = 255;
    }
    let before = flat(4, 4, 0);
    let report = diff(&before, &after, 4, 4, &cfg).expect("a report");
    assert_eq!(report.regions, vec![Rect::new(0, 0, 2, 4)]);
    assert_eq!(report.changed_fraction, 2.0 / 4.0);
}

#[test]
fn separate_changed_columns_stay_separate() {
    let cfg = DirtyConfig {
        grid_cols: 3,
        grid_rows: 1,
        ..config()
    };
    let mut after = flat(6, 1, 0);
    for x in [0, 5] {
        let base = x * RGBA_BYTES;
        after[base] = 255;
        after[base + 1] = 255;
        after[base + 2] = 255;
    }
    let before = flat(6, 1, 0);
    let report = diff(&before, &after, 6, 1, &cfg).expect("a report");
    assert_eq!(
        report.regions,
        vec![Rect::new(0, 0, 2, 1), Rect::new(4, 0, 2, 1)]
    );
}

#[test]
fn a_change_below_the_threshold_is_ignored() {
    let cfg = config();
    let before = flat(8, 8, 100);
    let after = flat(8, 8, 105);
    let report = diff(&before, &after, 8, 8, &cfg).expect("a report");
    assert_eq!(report.changed_fraction, 0.0);
    assert!(!report.significant);
}

#[test]
fn a_significance_floor_suppresses_a_single_cell() {
    let cfg = DirtyConfig {
        min_changed_cells: 5,
        ..config()
    };
    let before = flat(8, 8, 0);
    let after = quadrant(8, 8, 255);
    let report = diff(&before, &after, 8, 8, &cfg).expect("a report");
    assert_eq!(report.regions.len(), 1, "the region is still reported");
    assert!(
        !report.significant,
        "four changed cells are below the floor of five"
    );
}

#[test]
fn a_truncated_frame_is_rejected() {
    let cfg = config();
    let frame = flat(8, 8, 0);
    assert!(diff(&frame, &frame, 0, 8, &cfg).is_none());
    assert!(diff(&frame, &frame, 8, 0, &cfg).is_none());
    assert!(diff(&frame, &frame[..10], 8, 8, &cfg).is_none());
    assert!(cell_means(&frame[..10], 8, 8, &cfg).is_none());
}

#[test]
fn the_config_is_clamped() {
    let cfg = DirtyConfig {
        grid_cols: 0,
        grid_rows: 0,
        change_threshold: 0,
        min_changed_cells: 0,
        ..DirtyConfig::default()
    }
    .clamped();
    assert_eq!(cfg.grid_cols, DEFAULT_GRID_COLS);
    assert_eq!(cfg.grid_rows, DEFAULT_GRID_ROWS);
    assert_eq!(cfg.change_threshold, DEFAULT_CHANGE_THRESHOLD);
    assert_eq!(cfg.min_changed_cells, DEFAULT_MIN_CHANGED_CELLS);

    let huge = DirtyConfig {
        grid_cols: 4096,
        grid_rows: 4096,
        ..DirtyConfig::default()
    }
    .clamped();
    assert!(huge.cells() <= MAX_GRID_CELLS);
    assert!(huge.grid_cols > 0 && huge.grid_rows > 0);
}

#[test]
fn the_first_frame_always_triggers() {
    let mut trigger = ReanalysisTrigger::new(config());
    let frame = flat(8, 8, 0);
    let report = trigger
        .observe(&frame, 8, 8)
        .expect("the first frame needs analysis");
    assert_eq!(report.changed_fraction, 1.0);
    assert_eq!(report.regions, vec![Rect::new(0, 0, 8, 8)]);
    assert!(trigger.cooling_down());
}

#[test]
fn a_static_screen_never_retriggers() {
    let mut trigger = ReanalysisTrigger::new(config());
    let frame = flat(8, 8, 0);
    trigger.observe(&frame, 8, 8).expect("first frame");
    for _ in 0..10 {
        assert!(
            trigger.observe(&frame, 8, 8).is_none(),
            "an unchanged screen must not re-analyse"
        );
    }
    assert!(!trigger.cooling_down());
}

#[test]
fn a_moving_carousel_triggers_once_per_cooldown() {
    let cfg = DirtyConfig {
        cooldown_frames: 2,
        ..config()
    };
    let mut trigger = ReanalysisTrigger::new(cfg);
    let first = flat(8, 8, 0);
    trigger.observe(&first, 8, 8).expect("first frame");

    let moving = quadrant(8, 8, 255);
    assert!(
        trigger.observe(&moving, 8, 8).is_some(),
        "the first changed frame triggers"
    );
    assert!(trigger.cooling_down());
    assert!(
        trigger.observe(&moving, 8, 8).is_none(),
        "the cooldown suppresses the second"
    );
    assert!(
        trigger.observe(&moving, 8, 8).is_none(),
        "the cooldown suppresses the third"
    );
    assert!(
        trigger.observe(&moving, 8, 8).is_none(),
        "an unchanged carousel is static once the baseline catches up"
    );
    let later = quadrant(8, 8, 128);
    assert!(
        trigger.observe(&later, 8, 8).is_some(),
        "a fresh change after the cooldown fires"
    );
}

#[test]
fn the_baseline_follows_suppressed_frames() {
    let cfg = DirtyConfig {
        cooldown_frames: 1,
        ..config()
    };
    let mut trigger = ReanalysisTrigger::new(cfg);
    trigger.observe(&flat(8, 8, 0), 8, 8).expect("first");
    let moving = quadrant(8, 8, 255);
    trigger.observe(&moving, 8, 8).expect("change");
    assert!(trigger.observe(&moving, 8, 8).is_none());
    // The frame has been the baseline for two observations, so it is now static.
    assert!(
        trigger.observe(&moving, 8, 8).is_none(),
        "a stable carousel stops re-analysing"
    );
}

#[test]
fn a_size_change_resets_the_baseline() {
    let mut trigger = ReanalysisTrigger::new(config());
    trigger.observe(&flat(8, 8, 0), 8, 8).expect("first");
    assert!(
        trigger.observe(&flat(8, 8, 0), 16, 16).is_none(),
        "a size change is not a content change"
    );
    assert!(
        trigger.observe(&flat(16, 16, 0), 16, 16).is_some(),
        "the next frame is a first frame again"
    );
}

#[test]
fn reports_carry_no_pixels() {
    let cfg = config();
    let before = flat(8, 8, 0);
    let after = quadrant(8, 8, 255);
    let report = diff(&before, &after, 8, 8, &cfg).expect("a report");
    let dbg = format!("{report:?}");
    assert!(dbg.contains("changed_fraction"));
    assert!(!dbg.contains(CANARY));
    assert!(!dbg.contains("SECRET"));
}

#[test]
fn the_document_describes_the_trigger() {
    let doc = include_str!("../../../../docs/DIRTY_RECT.md");
    assert!(doc.contains("# Dirty-rect re-analysis"));
    assert!(doc.contains("cooldown"));
    assert!(doc.contains("ReanalysisTrigger"));
    assert!(!doc.contains(CANARY));
}
