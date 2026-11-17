# curbcut-render

Overlay plan types, hit-testing, and rendering contracts.

## Status

The focus surfaces are implemented: `ReadingRuler` highlights one to
three text lines and `FocusLens` magnifies a region with a pointer lead
and a jitter dead zone; both are disabled under reduced motion. The
animation policy caps every animation at 150 ms, applies an essential
change instantly under reduced motion, refuses decoration, and never
allows a vestibular trigger. See
[`../../docs/FOCUS.md`](../../docs/FOCUS.md) and
[`../../docs/ANIMATION.md`](../../docs/ANIMATION.md).

## Build and test

```bash
cargo test -p curbcut-render
cargo clippy -p curbcut-render --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
