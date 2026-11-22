# Layout solver

The layout solver turns a flat list of elements into a tile layout, and a list
of measured words into lines. It lives in `crates/curbcut-adapt/src/layout.rs`
and is pure: no I/O, no clock, no panics. `CC-063`.

## Rules

`place_tiles(viewport, elements, options)` applies three rules in order:

1. **Big targets first.** Elements are visited by descending `priority`, so a
   high-priority element claims its preferred size before lower-priority
   elements are considered.
2. **Reading order second.** Within a priority, elements are visited in
   ascending `reading_order`. Boxes fill each row left to right and rows top to
   bottom, so reading order is preserved spatially.
3. **No overlap, no clipping.** Boxes occupy disjoint rows. A box that does not
   fit the remaining row width wraps to a new row rather than being shrunk
   horizontally. A box that does not fit the viewport height is shrunk toward
   its minimum, and if it still does not fit it is dropped. A box whose minimum
   exceeds the viewport is dropped.

`LayoutSolution` reports both halves: `placed` boxes in placement order and
`dropped` element ids in reading order. `has_no_overlap()` and
`has_no_clipping(viewport)` are the invariants a caller can re-check, and
`accounts_for(elements)` confirms every input was either placed or dropped
exactly once.

A box is never resized below its `min` and never clipped.

Both checkers compute rectangle edges in `i64`. A viewport whose far edge
leaves the `i32` range cannot hold a rect that `Rect::right` and `Rect::bottom`
can describe, so `place_tiles` drops everything rather than emit one. The
solver is total: no input, including `i32::MAX` origins and `u32::MAX` sizes,
makes it panic.

## Sizes

`LayoutElement` carries a `min` and a `preferred` size. The solver clamps
`preferred` to the viewport, then raises it to `min`, so a caller that leaves
`preferred` empty gets the minimum. A minimum with a zero dimension, or one
larger than the viewport, is dropped rather than placed as a degenerate box.

## Reflow

`reflow(words, measure, space)` breaks measured words into lines:

- Words are visited in order and a line grows while
  `line width + space + word width <= measure`.
- A word that does not fit closes the line and starts a new one.
- A word wider than `measure` occupies a line of its own and that line is
  reported in `overflowing_lines`. The word is emitted whole: **reflowed text
  is never clipped**.
- A `measure` of zero puts every word on its own overflowing line rather than
  panicking.

`ReflowResult::preserves_words(count)` confirms every input word appears
exactly once and in order.

Line widths are `u64`. A line of very wide words cannot saturate and hide an
overflow, and a measure of zero puts every word on its own overflowing line
rather than panicking.

The style around a reflow — font fallback, line box, alignment, and WCAG AA
contrast — lives in `crates/curbcut-adapt/src/typography.rs`. See
[`TYPOGRAPHY.md`](TYPOGRAPHY.md).

## Performance

`ten_thousand_random_layouts_do_not_overlap_or_clip` runs 10,000 deterministic
pseudo-random viewports and element sets, asserts the invariants and the
reading-order rule on each, and checks that the 95th-percentile solve time is
under 50 ms. The test prints the measured placement counts and p95.

Measured on the development machine: 88,105 boxes placed and 81,337 dropped
across the 10,000 cases, with p95 solve time between 28 µs and 90 µs depending
on machine load — three orders of magnitude inside the gate.

## Privacy

The solver handles geometry only. Element ids are stable identifiers, not
content; no text, OCR output, or accessibility strings enter the solver, and
none are logged.
