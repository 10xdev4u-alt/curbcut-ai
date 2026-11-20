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

/// A frame with two vertical bands set to `value`.
fn bands(width: u32, height: u32, value: u8) -> Vec<u8> {
    let mut data = flat(width, height, 0);
    for y in 0..height as usize {
        for x in 0..width as usize {
            if x < 2 || x + 2 >= width as usize {
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
fn two_bands_on_both_rows_merge_into_two_regions() {
    let cfg = DirtyConfig {
        grid_cols: 4,
        grid_rows: 2,
        ..config()
    };
    let before = flat(8, 4, 0);
    let after = bands(8, 4, 255);
    let report = diff(&before, &after, 8, 4, &cfg).expect("a report");
    assert_eq!(
        report.regions,
        vec![Rect::new(0, 0, 2, 4), Rect::new(6, 0, 2, 4)],
        "each band is one region, not one per row"
    );
    assert_eq!(report.changed_fraction, 4.0 / 8.0);
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
fn a_region_on_a_frame_smaller_than_the_grid_is_not_empty() {
    let cfg = DirtyConfig {
        grid_cols: 16,
        grid_rows: 9,
        ..config()
    };
    let mut after = flat(8, 8, 0);
    after[0] = 255;
    after[1] = 255;
    after[2] = 255;
    let before = flat(8, 8, 0);
    let report = diff(&before, &after, 8, 8, &cfg).expect("a report");
    assert!(!report.regions.is_empty());
    assert!(
        report
            .regions
            .iter()
            .all(|rect| rect.width > 0 && rect.height > 0),
        "no zero-area rectangle may reach region-of-interest capture: {:?}",
        report.regions
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
fn the_grid_check_does_not_overflow() {
    // 65536 * 65536 wraps to zero in u32, so a u32 product would accept it.
    let cfg = DirtyConfig {
        grid_cols: 65536,
        grid_rows: 65536,
        ..DirtyConfig::default()
    }
    .clamped();
    assert!(cfg.cells() <= MAX_GRID_CELLS);
    assert!(cfg.grid_cols > 0 && cfg.grid_rows > 0);
}

#[test]
fn the_significance_floor_is_capped_to_the_grid() {
    let cfg = DirtyConfig {
        min_changed_cells: 5000,
        ..DirtyConfig::default()
    }
    .clamped();
    assert!(cfg.min_changed_cells <= cfg.cells());
    assert!(cfg.min_changed_cells > 0);
}

#[test]
fn the_public_diff_clamps_its_config() {
    let cfg = DirtyConfig {
        grid_cols: 0,
        grid_rows: 0,
        change_threshold: 0,
        ..DirtyConfig::default()
    };
    let frame = flat(8, 8, 200);
    let report = diff(&frame, &frame, 8, 8, &cfg).expect("a report");
    assert!(report.changed_fraction.is_finite());
    assert_eq!(report.changed_fraction, 0.0);
    assert!(
        !report.significant,
        "a zero threshold must not report every cell changed"
    );
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
fn a_continuously_moving_carousel_fires_once_per_cooldown_window() {
    let cfg = DirtyConfig {
        cooldown_frames: 2,
        ..config()
    };
    let mut trigger = ReanalysisTrigger::new(cfg);
    let first = flat(8, 8, 0);
    trigger.observe(&first, 8, 8).expect("first frame");

    let a = quadrant(8, 8, 255);
    let b = quadrant(8, 8, 128);
    let mut fires = 0;
    for step in 0..9 {
        let frame = if step % 2 == 0 { &a } else { &b };
        if trigger.observe(frame, 8, 8).is_some() {
            fires += 1;
        }
    }
    assert_eq!(
        fires, 3,
        "one fire every cooldown_frames + 1 frames, not one per frame"
    );
}

#[test]
fn a_cooldown_suppresses_a_significant_frame() {
    let cfg = DirtyConfig {
        cooldown_frames: 2,
        ..config()
    };
    let mut trigger = ReanalysisTrigger::new(cfg);
    trigger.observe(&flat(8, 8, 0), 8, 8).expect("first frame");
    let a = quadrant(8, 8, 255);
    let b = quadrant(8, 8, 128);
    // The first frame armed a cooldown of two, so the next two frames are
    // suppressed even though each is a significant change.
    assert!(
        trigger.observe(&a, 8, 8).is_none(),
        "suppressed while cooling"
    );
    assert!(trigger.observe(&b, 8, 8).is_none(), "still cooling");
    assert!(
        trigger.observe(&a, 8, 8).is_some(),
        "fires after the cooldown"
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
    assert!(
        trigger.observe(&moving, 8, 8).is_none(),
        "the first change is suppressed by the cooldown, but the baseline follows it"
    );
    assert!(
        trigger.observe(&moving, 8, 8).is_none(),
        "the carousel is now the baseline, so it is static"
    );
    assert!(
        trigger.observe(&flat(8, 8, 0), 8, 8).is_some(),
        "a fresh change fires"
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
fn the_trigger_debug_prints_the_baseline_shape_not_its_bytes() {
    let mut trigger = ReanalysisTrigger::new(config());
    let frame = quadrant(8, 8, 255);
    trigger.observe(&frame, 8, 8).expect("first");
    let dbg = format!("{trigger:?}");
    assert!(dbg.contains("ReanalysisTrigger"));
    assert!(dbg.contains("cooldown_left"));
    assert!(dbg.contains("baseline"));
    assert!(
        !dbg.contains(CANARY) && !dbg.contains("SECRET"),
        "the baseline is a downscaled proxy of the screen and must not print"
    );
}

#[test]
fn the_document_describes_the_trigger() {
    let doc = include_str!("../../../../docs/DIRTY_RECT.md");
    assert!(doc.contains("# Dirty-rect re-analysis"));
    assert!(doc.contains("cooldown"));
    assert!(doc.contains("ReanalysisTrigger"));
    assert!(!doc.contains(CANARY));
}
