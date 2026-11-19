# Vision pipeline

The vision crate decides which frames are worth a model call, then runs
detection, OCR, and icon captioning. All inference is on-device.

## Stages

| Module | Item | Role |
| --- | --- | --- |
| `frame` | `VisionFrame` | A borrowed frame: metadata, pixels, and a protected flag. |
| `motion` | `MotionSignature` | A fixed-size sample of a frame's pixels. |
| `scheduler` | `Scheduler`, `SkipReason`, `SchedulerStats` | The motion-aware decision. |
| `detector` | `Detector`, `Detection`, `NoDetector` | Element detection. |
| `ocr` | `Ocr`, `OcrText`, `NoOcr` | Text recognition. |
| `caption` | `IconCaptioner`, `IconCaption`, `NoCaptioner` | Icon and image captions. |
| `layout` | `LayoutAnalyzer`, `LayoutAnalysis`, `AnalyzedRegion` | Regions and reading order. |
| `pipeline` | `VisionPipeline`, `VisionOutcome`, `VisionResult` | The composed entry point. |

## Scheduling

`Scheduler::decide` returns `None` (analyze) or a `SkipReason`:

| Reason | Meaning |
| --- | --- |
| `Protected` | The frame is protected. Never analyzed, and scheduler state does not advance. |
| `Static` | Motion against the previous frame is below the threshold. |
| `Coalesced` | Motion is above the threshold, but a frame was analyzed inside the coalescing window. |

Rules:

- The first unprotected frame is always analyzed.
- Motion is the fraction of sampled bytes that changed since the previous
  unprotected frame. It is frame-to-frame, so a slow drift below the threshold
  stays skipped.
- A moving frame within `coalesce` of the last analyzed frame is coalesced, so a
  burst produces one analysis.
- Threshold is clamped to `0.0..=1.0`; the default is `0.08`. The default
  coalescing window is 500 ms.

The scheduler reads `FrameMeta::captured_at`, never the wall clock, so a run is
reproducible. `decisions_are_deterministic` pins that.

Protected frames never reach a model. `a_protected_frame_is_never_enqueued`
asserts the detector was not called at all.

## Pipeline

```rust
let mut pipeline = VisionPipeline::new(
    Box::new(detector),
    Box::new(ocr),
    Box::new(captioner),
);
match pipeline.process(&frame) {
    VisionOutcome::Analyzed(result) => { /* result.detections, result.texts, result.captions */ }
    VisionOutcome::Skipped(reason) => { /* no model call */ }
}
```

A stage that fails records its name in `VisionResult::errors` and contributes an
empty result. The pipeline does not panic and does not stop the other stages.

`VisionPipeline::passive()` wires `NoDetector`, `NoOcr`, and `NoCaptioner`.

## Privacy

`OcrText` and `IconCaption` hold content. Their `Debug` impls print lengths, not
text, and `redacted()` returns a content-free view. Nothing in this crate logs
pixels, OCR text, or captions. `a_result_carries_no_content_in_its_debug` pins
it. Full frame pixels are borrowed and never retained; the scheduler keeps only
a 32-byte `MotionSignature`, which is zeroized on drop.

## Testing

```bash
cargo test -p curbcut-vision
cargo clippy -p curbcut-vision --all-targets -- -D warnings
```

Unit tests cover the scheduler's decision table and the signature math. The
integration tests in `tests/vision_pipeline.rs` drive the public API the way the
capture adapter does, including a detector that fails the test if a protected
frame reaches it.

## Layout

`LayoutAnalyzer::analyze(graph)` divides the viewport into regions and returns
a reading order. See [`LAYOUT_ANALYSIS.md`](LAYOUT_ANALYSIS.md).

- Header and footer are wide, shallow bands anchored to the top and bottom.
- Nav and aside are narrow, tall columns anchored to the left and right.
- Main is the residual centre; a modal is a small off-edge surface that is
  centred or holds a dialog role; otherwise it is floating.
- A region at least `containment_ratio` covered by a larger region in the same
  layer is dropped, and every element is assigned to the region it overlaps
  most. The largest region is never a modal.
- Reading order is header, nav, main, aside, footer, modal, floating; elements
  inside a region are ordered by `(y, x, id)`.
