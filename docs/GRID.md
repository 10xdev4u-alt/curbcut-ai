# Icon grid

The icon grid replaces a dense menu or toolbar with large labelled tiles. The
pure half — layout, label policy, and search — lives in
`crates/curbcut-adapt/src/grid.rs`. The renderer draws the rectangles and the
caller supplies the labels. `CC-056`.

## Layout

`icon_grid_layout(viewport, tiles, settings)` takes a viewport `Rect`, a slice
of `GridTile` (id, label, reading order), and the profile's `IconGrid` settings
(`min_tile`, `columns`, `symbols`, `gap`). It returns a `GridLayout` with:

- `columns` — the column count the viewport could honour;
- `tile` — the square tile size;
- `placed` — `GridPlacement` rectangles in row-major reading order;
- `dropped` — tile ids that did not fit, in reading order.

Three rules hold:

1. **A tile is never smaller than `icon_grid.min_tile`.** The column count
   starts at `min(settings.columns, tile count)` and is reduced until the
   tile width reaches the minimum. The tile is square, so its side is also
   capped by the viewport height. `IconGrid::validate` requires
   `min_tile >= 16`, but the solver takes any `IconGrid`, so a zero minimum is
   floored to 1; otherwise a zero-area tile would be placed and the placed set
   would stop being a reading-order prefix.
2. **Reading order is row-major.** Tiles are visited by
   `(reading_order, id)` and placed left to right, top to bottom.
3. **A tile that does not fit is dropped.** Tiles never overlap and never
   leave the viewport.

If the viewport cannot fit even one minimum tile — too narrow, or too short for
a square tile — every tile is dropped rather than clipped. An empty tile list
produces an empty layout with `columns == 0`.

The tile side is also capped at `i32::MAX`. `Rect::right` and `Rect::bottom`
add `width as i32`, so a side above `i32::MAX` would wrap; the cap keeps every
emitted rectangle describable by the core type.

The default profile sets `icon_grid.min_tile = 72`, which is the product rule
that a tile is at least 72 px. A user can lower it to 16 in the profile; the
solver honours whatever the profile says.

`GridLayout::has_no_overlap`, `has_no_clipping`, and `accounts_for` let a
caller re-check a layout, and the tests feed deliberately broken layouts to
them so the checks cannot be vacuous.

## Labels

`validate_label` enforces the label policy:

- non-empty;
- plain text on one line: no control character and no invisible format character
  (the soft hyphen, zero-width spaces and joiners, the line and paragraph
  separators, the bidi overrides and isolates, and the byte-order mark);
- at most `MAX_LABEL_WORDS` (8) words.

`MAX_LABEL_WORDS` is defined in `curbcut_core::types` and re-exported here, so
the icon-grid policy and `curbcut_core::types::Tile` cannot drift.

`label_word_count` counts whitespace-separated words and `is_plain_label` is
the boolean form. The layout itself is geometric and does not inspect labels;
the caller validates a label before it becomes a tile.

## Search

`search_tiles(tiles, query)` filters tiles without reordering them:

- the query is split on whitespace and lowercased;
- a tile matches when **every** token appears in its lowercased label, so
  `new file` matches `New file` and `Create a new file`, and `file new` is the
  same query;
- an empty or whitespace-only query returns every tile;
- the result is in `(reading_order, id)` order and never duplicates a tile.

## Privacy

Labels are content. This module never logs a label: it counts words and
compares tokens, and the only thing it returns is element ids and rectangles.
The `Debug` impl on `GridTile` redacts the label to a length, so a caller that
`dbg!`s or traces a tile cannot leak it. The workspace canary scan covers this
crate.
