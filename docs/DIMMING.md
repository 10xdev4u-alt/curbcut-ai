# Dimming with except-regions

`dim_plan` in `crates/curbcut-adapt/src/dim.rs` computes the rectangles that
cover a screen except the focused control and the profile's except-regions.
The renderer fills those rectangles with a scrim at the profile opacity.
`CC-055`.

## Rules

1. **The focused control is never dimmed.** It is protected first, and every
   except-region is protected too. `DimPlan::does_not_cover_any` re-checks it.
2. **The dim is exact.** The dim rectangles cover the screen minus the
   protected rectangles, with no overlap and no gap.
   `DimPlan::covers_exactly` asserts the area identity
   `scrim area + protected union area = screen area`, and the tests sample the
   plane to confirm no point is dimmed twice or dimmed while protected.
3. **Reduced motion is static.** `DimMotion::Static` applies the scrim
   immediately; `DimMotion::Fade { duration_ms }` carries a duration the
   renderer may honour. `DimPlan::is_static` reports which one it is.

## Inputs

`DimRequest` carries the screen `Rect`, the focused control as an `Option<Rect>`,
the except-regions as a slice of `Rect`, the opacity as an `f32`, and the
motion. `dim_plan` returns `DimError::OpacityOutOfRange` when the opacity is
outside `0.0..=1.0`; every other input is total.

Protected rectangles are clipped to the screen, so a region that is mostly
off-screen protects only the part that is visible, and a region entirely
outside contributes nothing.

## Output

`DimPlan` carries:

- `rects` — the scrim rectangles, merged vertically where they share an
  x-range;
- `opacity` — the validated opacity;
- `motion` — `Static` or `Fade`;
- `protected` — the clipped protected rectangles.

## How the complement is computed

The screen is split into cells at every protected edge. A two-dimensional
difference array marks the cells covered by any protected rectangle; because
the split uses exact coordinates, a cell is either fully inside or fully
outside each rectangle. Unprotected runs are emitted row by row and then
merged vertically. This is deterministic and avoids the off-by-one errors a
hand-written band splitter invites.

`union_area` computes the union area of a set of rectangles with a sweep over
y-slabs. It is used by the exactness check and is useful on its own.

## Bounds and extremes

The profile bounds `dim.except` to 64 regions, so a plan is computed from at
most 65 protected rectangles. The screen is clamped to the range `Rect` can
describe: a screen whose far edge exceeds `i32::MAX` is trimmed, and a screen
that cannot be represented at all yields an empty plan rather than a panic.

## Privacy

The module handles geometry only. It never logs a rectangle's origin or size
beyond the plan itself, and it never sees screen content, OCR text, or
accessibility strings. The workspace canary scan covers this crate.
