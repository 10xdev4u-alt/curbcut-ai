# Dirty-rect re-analysis

## Why

A model call is the most expensive thing CurbCut does. A static screen does not
need another one; a moving carousel does. The trigger's job is to spend a
cheap frame diff so the expensive analysis runs only when the screen actually
changed.

## The rule

`diff` reduces two RGBA frames to a grid of cell means and reports the cells
whose mean luma moved. `ReanalysisTrigger::observe` consumes one frame at a
time and returns a `ChangeReport` only when re-analysis should run.

- **The grid is a downscaled proxy.** `DirtyConfig::grid_cols` and
  `grid_rows` (defaults 16 x 9) divide the frame; each cell's mean Rec.709
  luma is one byte. A cell is changed when the absolute difference of the two
  means reaches `change_threshold` (default 12).
- **Changed cells merge into rectangles.** Horizontal runs merge first, then
  runs on consecutive rows spanning the same columns merge vertically. The
  result is a small set of pixel-space `Rect`s a caller can hand to a
  region-of-interest capture.
- **Significance is a floor, not a ratio.** A frame is significant when at
  least `min_changed_cells` cells changed (default 1). The floor exists so a
  single noisy cell can be ignored without tuning a fraction.
- **A cooldown collapses a burst.** After a trigger the trigger sleeps for
  `cooldown_frames` frames (default 1), so a carousel animating at 30 fps causes
  one re-analysis, not thirty.
- **The baseline follows every frame**, including the ones the cooldown
  suppressed. A carousel that stops moving becomes static and stops
  re-analysing.
- **The first frame always triggers**: there is no baseline yet.
- A size change or a truncated frame resets the baseline; the next good frame
  is treated as a first frame.

## API

```rust
use curbcut_core::types::Rect;
use curbcut_vision::dirty::{self, DirtyConfig, ReanalysisTrigger};

let config = DirtyConfig::default();
let report = dirty::diff(previous, next, width, height, &config);
// report.regions: Vec<Rect>, report.significant: bool

let mut trigger = ReanalysisTrigger::new(config);
if let Some(report) = trigger.observe(frame, width, height) {
    // re-analyse report.regions
}
```

## Idle cost

The trigger reads one frame and keeps `grid_cols * grid_rows` bytes of cell
means. It never retains pixels and never calls a model. The project's idle-CPU
target (< 5%) is a benchmark measurement, not a claim made here; this module is
the piece that makes it reachable by skipping analysis on an unchanged screen.

## Privacy

Only cell means are stored. A `ChangeReport` carries dimensions, a fraction,
rectangles, and a flag — never pixels, OCR text, or accessibility strings. The
crate's no-content-logging canary enforces this.

## Tests

`crates/curbcut-vision/src/dirty/tests.rs` covers:

- an identical frame producing no regions;
- one changed quadrant becoming one rectangle;
- vertical merging of adjacent changed rows and separation of distinct columns;
- a change below the threshold and a significance floor;
- truncated frames and zero dimensions rejected;
- config clamping, including an oversized grid;
- the first frame triggering, a static screen never retriggering, a moving
  carousel triggering once per cooldown, and the baseline following
  suppressed frames;
- a size change resetting the baseline;
- report and document canaries.
