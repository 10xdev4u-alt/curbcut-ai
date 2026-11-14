# curbcut-vision

ONNX inference, detection, OCR, layout, and motion scheduling.

## Status

The vision pipeline is implemented: Detector, Ocr, and IconCaptioner
traits, a deterministic motion-aware scheduler, and VisionPipeline.
See [`../../docs/VISION.md`](../../docs/VISION.md).

## Build and test

```bash
cargo test -p curbcut-vision
cargo clippy -p curbcut-vision --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
