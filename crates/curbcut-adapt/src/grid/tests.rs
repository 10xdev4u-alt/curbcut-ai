//! Tests for the icon grid.

use super::*;

fn tile(id: u64, reading_order: u32) -> GridTile {
    GridTile::new(ElementId(id), format!("Tile {id}"), reading_order)
}

fn settings(columns: u32, min_tile: u32, gap: u32) -> IconGrid {
    IconGrid {
        columns,
        min_tile,
        gap,
        symbols: false,
    }
}

#[test]
fn tiles_fill_rows_left_to_right_and_top_to_bottom() {
    let viewport = Rect::new(0, 0, 400, 300);
    let tiles = [
        tile(1, 0),
        tile(2, 1),
        tile(3, 2),
        tile(4, 3),
        tile(5, 4),
        tile(6, 5),
    ];
    let grid = settings(3, 72, 12);

    let layout = icon_grid_layout(viewport, &tiles, &grid);

    // (400 - 2*12) / 3 = 125, step 137.
    assert_eq!(layout.columns, 3);
    assert_eq!(layout.tile, Size::new(125, 125));
    assert_eq!(layout.dropped, Vec::new());
    assert_eq!(layout.get(ElementId(1)), Some(Rect::new(0, 0, 125, 125)));
    assert_eq!(layout.get(ElementId(2)), Some(Rect::new(137, 0, 125, 125)));
    assert_eq!(layout.get(ElementId(3)), Some(Rect::new(274, 0, 125, 125)));
    assert_eq!(layout.get(ElementId(4)), Some(Rect::new(0, 137, 125, 125)));
    assert_eq!(
        layout.get(ElementId(6)),
        Some(Rect::new(274, 137, 125, 125))
    );
    assert!(layout.has_no_overlap());
    assert!(layout.has_no_clipping(viewport));
    assert!(layout.accounts_for(&tiles));
}

#[test]
fn a_single_tile_is_capped_by_the_viewport_height() {
    // One column would make the tile as wide as the viewport, which would not
    // fit vertically. The square tile is capped by the height instead.
    let viewport = Rect::new(0, 0, 400, 300);
    let tiles = [tile(1, 0)];

    let layout = icon_grid_layout(viewport, &tiles, &settings(3, 72, 12));

    assert_eq!(layout.columns, 1);
    assert_eq!(layout.tile, Size::new(300, 300));
    assert_eq!(layout.get(ElementId(1)), Some(Rect::new(0, 0, 300, 300)));
    assert!(layout.has_no_clipping(viewport));
}

#[test]
fn columns_are_capped_by_the_tile_count() {
    let viewport = Rect::new(0, 0, 600, 400);
    let tiles = [tile(1, 0), tile(2, 1)];

    let layout = icon_grid_layout(viewport, &tiles, &settings(6, 40, 8));

    assert_eq!(layout.columns, 2, "two tiles cannot fill six columns");
    assert_eq!(layout.tile, Size::new(296, 296));
    assert_eq!(layout.len(), 2);
}

#[test]
fn a_narrow_viewport_reduces_the_column_count_to_honour_the_minimum() {
    let viewport = Rect::new(0, 0, 100, 300);
    let tiles = [tile(1, 0), tile(2, 1), tile(3, 2)];

    let layout = icon_grid_layout(viewport, &tiles, &settings(3, 72, 12));

    // 3 columns would give 25 px tiles, 2 would give 44, so it falls to one
    // 100 px column. The third tile does not fit the 300 px height.
    assert_eq!(layout.columns, 1);
    assert_eq!(layout.tile, Size::new(100, 100));
    assert_eq!(layout.len(), 2);
    assert_eq!(layout.dropped, vec![ElementId(3)]);
    assert!(layout.has_no_overlap());
    assert!(layout.has_no_clipping(viewport));
    assert!(layout.accounts_for(&tiles));
}

#[test]
fn a_viewport_too_narrow_for_the_minimum_drops_everything() {
    let viewport = Rect::new(0, 0, 50, 400);
    let tiles = [tile(1, 0), tile(2, 1)];

    let layout = icon_grid_layout(viewport, &tiles, &settings(2, 72, 8));

    assert!(layout.is_empty());
    assert_eq!(layout.dropped, vec![ElementId(1), ElementId(2)]);
    assert!(layout.accounts_for(&tiles));
}

#[test]
fn the_default_settings_give_tiles_of_at_least_72_points() {
    // The product rule is a tile of at least 72 px. `icon_grid.min_tile`
    // defaults to 72, so a normal viewport honours it.
    let viewport = Rect::new(0, 0, 400, 300);
    let tiles = [tile(1, 0), tile(2, 1), tile(3, 2)];

    let layout = icon_grid_layout(viewport, &tiles, &IconGrid::default());

    assert!(layout.tile.width >= 72, "tile was {} px", layout.tile.width);
    assert!(
        layout.tile.height >= 72,
        "tile was {} px",
        layout.tile.height
    );
    assert_eq!(layout.tile.width, 125);
}

#[test]
fn a_tile_that_does_not_fit_the_height_is_dropped_in_reading_order() {
    let viewport = Rect::new(0, 0, 200, 100);
    let tiles = [tile(1, 0), tile(2, 1), tile(3, 2), tile(4, 3)];

    let layout = icon_grid_layout(viewport, &tiles, &settings(2, 40, 8));

    assert_eq!(layout.len(), 2);
    assert_eq!(layout.dropped, vec![ElementId(3), ElementId(4)]);
    assert!(layout.has_no_clipping(viewport));
    assert!(layout.has_no_overlap());
}

#[test]
fn reading_order_wins_over_id_order() {
    let viewport = Rect::new(0, 0, 400, 200);
    let tiles = [
        GridTile::new(ElementId(9), "Ninth", 1),
        GridTile::new(ElementId(2), "Second", 0),
    ];

    let layout = icon_grid_layout(viewport, &tiles, &settings(2, 40, 8));

    let first = layout.get(ElementId(2)).expect("placed");
    let second = layout.get(ElementId(9)).expect("placed");
    assert!(first.x < second.x, "reading order 0 then 1");
}

#[test]
fn an_empty_tile_list_is_empty() {
    let viewport = Rect::new(0, 0, 400, 300);
    let layout = icon_grid_layout(viewport, &[], &IconGrid::default());
    assert!(layout.is_empty());
    assert_eq!(layout.columns, 0);
    assert_eq!(layout.tile, Size::new(0, 0));
}

#[test]
fn extreme_viewport_edges_are_total() {
    let tiles = [tile(1, 0)];
    for viewport in [
        Rect::new(i32::MAX, 0, 10, 10),
        Rect::new(i32::MIN, 0, 10, 10),
        Rect::new(0, i32::MAX, 10, 10),
        Rect::new(0, 0, u32::MAX, 10),
        Rect::new(0, 0, 10, u32::MAX),
    ] {
        let layout = icon_grid_layout(viewport, &tiles, &IconGrid::default());
        for placed in &layout.placed {
            assert!(i64::from(placed.rect.x) + i64::from(placed.rect.width) <= i64::from(i32::MAX));
            assert!(
                i64::from(placed.rect.y) + i64::from(placed.rect.height) <= i64::from(i32::MAX)
            );
        }
        assert!(layout.has_no_overlap());
        assert!(layout.has_no_clipping(viewport));
        assert!(layout.accounts_for(&tiles));
    }
}

#[test]
fn the_grid_checkers_reject_broken_layouts() {
    let tiles = [tile(1, 0), tile(2, 1)];
    let viewport = Rect::new(0, 0, 200, 200);

    let overlapping = GridLayout {
        columns: 2,
        tile: Size::new(100, 100),
        placed: vec![
            GridPlacement {
                id: ElementId(1),
                rect: Rect::new(0, 0, 100, 100),
            },
            GridPlacement {
                id: ElementId(2),
                rect: Rect::new(50, 50, 100, 100),
            },
        ],
        dropped: Vec::new(),
    };
    assert!(!overlapping.has_no_overlap());

    let clipped = GridLayout {
        columns: 1,
        tile: Size::new(100, 100),
        placed: vec![GridPlacement {
            id: ElementId(1),
            rect: Rect::new(150, 150, 100, 100),
        }],
        dropped: vec![ElementId(2)],
    };
    assert!(!clipped.has_no_clipping(viewport));

    let incomplete = GridLayout {
        columns: 2,
        tile: Size::new(100, 100),
        placed: vec![GridPlacement {
            id: ElementId(1),
            rect: Rect::new(0, 0, 100, 100),
        }],
        dropped: Vec::new(),
    };
    assert!(!incomplete.accounts_for(&tiles));

    let duplicated = GridLayout {
        columns: 2,
        tile: Size::new(100, 100),
        placed: vec![
            GridPlacement {
                id: ElementId(1),
                rect: Rect::new(0, 0, 100, 100),
            },
            GridPlacement {
                id: ElementId(1),
                rect: Rect::new(100, 0, 100, 100),
            },
        ],
        dropped: vec![ElementId(2)],
    };
    assert!(!duplicated.accounts_for(&tiles));

    let valid = GridLayout {
        columns: 2,
        tile: Size::new(100, 100),
        placed: vec![
            GridPlacement {
                id: ElementId(1),
                rect: Rect::new(0, 0, 100, 100),
            },
            GridPlacement {
                id: ElementId(2),
                rect: Rect::new(100, 0, 100, 100),
            },
        ],
        dropped: Vec::new(),
    };
    assert!(valid.has_no_overlap());
    assert!(valid.has_no_clipping(viewport));
    assert!(valid.accounts_for(&tiles));
}

#[test]
fn labels_are_plain_and_short() {
    assert_eq!(label_word_count("Save"), 1);
    assert_eq!(label_word_count("  New   file  "), 2);
    assert_eq!(label_word_count(""), 0);

    assert!(is_plain_label("Save"));
    assert!(is_plain_label("Open the file in a new window please"));
    assert!(!is_plain_label(""));
    assert!(!is_plain_label("   "));
    assert!(!is_plain_label("Save\nAs"));
    assert!(!is_plain_label("Open the file in a new window please now"));
}

#[test]
fn label_validation_reports_the_problem() {
    assert_eq!(validate_label("Save"), Ok(()));
    assert_eq!(validate_label(""), Err(LabelError::Empty));
    assert_eq!(validate_label("  "), Err(LabelError::Empty));
    assert_eq!(validate_label("Save\nAs"), Err(LabelError::NotPlain));
    assert_eq!(
        validate_label("one two three four five six seven eight nine"),
        Err(LabelError::TooManyWords)
    );
    assert_eq!(
        validate_label("one two three four five six seven eight"),
        Ok(())
    );
    assert!(LabelError::TooManyWords.to_string().contains('8'));
}

#[test]
fn search_matches_every_token_case_insensitively() {
    let tiles = [
        GridTile::new(ElementId(1), "New file", 0),
        GridTile::new(ElementId(2), "Open file", 1),
        GridTile::new(ElementId(3), "Save", 2),
        GridTile::new(ElementId(4), "New folder", 3),
    ];

    assert_eq!(
        search_tiles(&tiles, "new"),
        vec![ElementId(1), ElementId(4)]
    );
    assert_eq!(search_tiles(&tiles, "NEW FILE"), vec![ElementId(1)]);
    assert_eq!(search_tiles(&tiles, "file new"), vec![ElementId(1)]);
    assert_eq!(
        search_tiles(&tiles, "file"),
        vec![ElementId(1), ElementId(2)]
    );
    assert_eq!(search_tiles(&tiles, "zip"), Vec::new());
}

#[test]
fn search_with_an_empty_query_returns_everything_in_reading_order() {
    let tiles = [
        GridTile::new(ElementId(9), "Ninth", 2),
        GridTile::new(ElementId(2), "Second", 0),
        GridTile::new(ElementId(5), "Fifth", 1),
    ];

    assert_eq!(
        search_tiles(&tiles, ""),
        vec![ElementId(2), ElementId(5), ElementId(9)]
    );
    assert_eq!(
        search_tiles(&tiles, "   "),
        vec![ElementId(2), ElementId(5), ElementId(9)]
    );
}

#[test]
fn search_never_reorders_or_duplicates() {
    let tiles = [
        GridTile::new(ElementId(1), "Alpha", 0),
        GridTile::new(ElementId(2), "Alpha beta", 1),
        GridTile::new(ElementId(3), "Alpha gamma", 2),
    ];

    let first = search_tiles(&tiles, "alpha");
    assert_eq!(first, vec![ElementId(1), ElementId(2), ElementId(3)]);
    assert_eq!(
        first.len(),
        first
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    assert_eq!(
        search_tiles(&tiles, "alpha"),
        first,
        "search is deterministic"
    );
}

#[test]
fn search_deduplicates_repeated_ids() {
    let tiles = [
        GridTile::new(ElementId(1), "Alpha", 0),
        GridTile::new(ElementId(1), "Alpha", 1),
        GridTile::new(ElementId(2), "Alpha", 2),
    ];

    assert_eq!(
        search_tiles(&tiles, "alpha"),
        vec![ElementId(1), ElementId(2)]
    );
    assert_eq!(search_tiles(&tiles, ""), vec![ElementId(1), ElementId(2)]);
}

#[test]
fn grid_tile_debug_redacts_the_label() {
    let tile = GridTile::new(ElementId(7), "SECRET_LABEL", 3);
    let debug = format!("{tile:?}");

    assert!(!debug.contains("SECRET_LABEL"), "{debug}");
    assert!(debug.contains("label_len"), "{debug}");
    assert!(debug.contains("reading_order"), "{debug}");
    assert!(debug.contains("ElementId(7)"), "{debug}");
}

#[test]
fn accounts_for_rejects_an_unknown_id() {
    let tiles = [tile(1, 0), tile(1, 1)];
    let layout = GridLayout {
        columns: 2,
        tile: Size::new(100, 100),
        placed: vec![
            GridPlacement {
                id: ElementId(1),
                rect: Rect::new(0, 0, 100, 100),
            },
            GridPlacement {
                id: ElementId(999),
                rect: Rect::new(100, 0, 100, 100),
            },
        ],
        dropped: Vec::new(),
    };

    assert!(!layout.accounts_for(&tiles));
}

#[test]
fn a_zero_minimum_still_places_a_reading_order_prefix() {
    let viewport = Rect::new(0, 0, 1, 1);
    let tiles = [tile(1, 0), tile(2, 1), tile(3, 2), tile(4, 3)];

    let layout = icon_grid_layout(viewport, &tiles, &settings(3, 0, 1));

    assert_eq!(layout.columns, 1);
    assert_eq!(layout.tile, Size::new(1, 1));
    assert_eq!(layout.len(), 1);
    assert_eq!(
        layout.dropped,
        vec![ElementId(2), ElementId(3), ElementId(4)]
    );
    assert!(layout.accounts_for(&tiles));
}

#[test]
fn extreme_negative_origin_with_a_huge_viewport_does_not_overflow() {
    let viewport = Rect::new(i32::MIN, i32::MIN, u32::MAX, u32::MAX);
    let tiles = [tile(1, 0)];

    let layout = icon_grid_layout(viewport, &tiles, &settings(1, 72, 0));

    for placed in &layout.placed {
        // `Rect::right`/`bottom` add in `i32`; a wrap would fail this equality.
        assert_eq!(
            i64::from(placed.rect.right()),
            i64::from(placed.rect.x) + i64::from(placed.rect.width)
        );
        assert_eq!(
            i64::from(placed.rect.bottom()),
            i64::from(placed.rect.y) + i64::from(placed.rect.height)
        );
    }
    assert!(layout.tile.width <= i32::MAX as u32);
    assert!(layout.tile.height <= i32::MAX as u32);
    assert!(layout.has_no_clipping(viewport));
    assert!(layout.accounts_for(&tiles));
}

#[test]
fn label_validation_rejects_invisible_format_characters() {
    assert_eq!(validate_label("Save"), Ok(()));
    for hidden in [
        '\u{00AD}', '\u{200B}', '\u{200C}', '\u{200D}', '\u{2028}', '\u{2029}', '\u{202A}',
        '\u{202E}', '\u{2060}', '\u{2066}', '\u{2069}', '\u{FEFF}',
    ] {
        let label = format!("a{hidden}b");
        assert_eq!(
            validate_label(&label),
            Err(LabelError::NotPlain),
            "{hidden:?} is not plain"
        );
        assert!(!is_plain_label(&label));
    }
}

#[test]
fn the_word_limit_is_shared_with_core() {
    assert_eq!(MAX_LABEL_WORDS, curbcut_core::types::MAX_LABEL_WORDS);
    assert_eq!(MAX_LABEL_WORDS, 8);
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/GRID.md");
    for rule in [
        "min_tile",
        "row-major",
        "dropped",
        "eight",
        "redact",
        "format character",
        "i32",
    ] {
        assert!(doc.contains(rule), "docs/GRID.md does not state `{rule}`");
    }
}
