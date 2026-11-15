# Reading ruler and focus lens

`crates/curbcut-render/src/focus.rs` implements the two focus surfaces: a
reading ruler that highlights one to three text lines, and a lens that
magnifies a region and leads the pointer slightly. `CC-058`.

## Rules

1. **Both are disabled under reduced motion.** `MotionMode::Reduced` makes
   `ReadingRuler::rect` and `FocusLens::update` return `None`; the caller hides
   the surface. `MotionMode::Full` is the default.
2. **The lens leads, then settles.** The target centre is the pointer plus a
   lead in the direction of travel, clamped to the screen. The centre moves
   halfway to the target, so motion eases instead of jumping.
3. **No jitter.** A pointer move inside the dead zone leaves the centre
   untouched, so a trembling pointer does not shake the lens.

## Types

- `MotionMode { Full, Reduced }` — `allows_ruler` and `allows_lens` are the
  predicates the surfaces call.
- `ReadingRuler` — `new(line_count, line_height, motion)`, `line_count`,
  `line_height`, `set_line_count`, `band_height`, `move_to`, `rect`, `motion`,
  `set_motion`. `line_count` is clamped to `1..=3`; `rect` spans the screen
  width and is clamped to the screen.
- `FocusLens` — `new(magnification, lead_px, dead_zone_px, motion)`,
  `magnification`, `set_magnification`, `lead_px`, `set_lead_px`,
  `dead_zone_px`, `center`, `update`, `source_rect`, `motion`, `set_motion`.
  Magnification is clamped to `1.0..=8.0`, lead and dead zone to `0..=64`.

`source_rect` is the screen region the lens magnifies: a square of side
`screen.width / magnification`, centred on the lens and clamped. A higher
magnification reads a smaller region.

## Privacy

Both surfaces store geometry only: line counts, pixel sizes, a centre, and the
last pointer. They never store screen content, OCR text, or accessibility
strings, and they never log a position alongside content. The workspace canary
scan covers this crate.
