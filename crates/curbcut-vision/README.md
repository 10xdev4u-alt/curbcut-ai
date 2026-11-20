# curbcut-vision

ONNX inference, detection, OCR, layout, and motion scheduling.

## Status

The vision pipeline is implemented: Detector, Ocr, and IconCaptioner
traits, a deterministic motion-aware scheduler, and VisionPipeline.
Motion classification reports a region as static, carousel, autoplay,
or countdown once it has moved for long enough. Layout analysis divides
a viewport into header, nav, main, aside, footer, modal, and floating
regions and produces a stable reading order. Dirty-rect re-analysis
reduces two frames to a grid of cell means, merges the changed cells
into rectangles, and triggers re-analysis with a cooldown so a moving
carousel causes one analysis rather than one per frame. See
[`../../docs/VISION.md`](../../docs/VISION.md),
[`../../docs/MOTION_DETECTION.md`](../../docs/MOTION_DETECTION.md),
[`../../docs/LAYOUT_ANALYSIS.md`](../../docs/LAYOUT_ANALYSIS.md), and
[`../../docs/DIRTY_RECT.md`](../../docs/DIRTY_RECT.md).

## Build and test

```bash
cargo test -p curbcut-vision
cargo clippy -p curbcut-vision --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
