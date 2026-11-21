# curbcut-capture

Screen capture adapters and the zeroizing frame ring buffer.

## Status

The zeroizing frame ring buffer is implemented: a fixed-capacity
ring with a byte budget, best-effort `mlock`, and a guard byte on
release. Protected-content detection refuses a DRM surface, a
secure desktop, an uncapturable window, or a black frame, and
returns the accessibility-only fallback message. See
[`../../docs/PROTECTED_CONTENT.md`](../../docs/PROTECTED_CONTENT.md).

## Build and test

```bash
cargo test -p curbcut-capture
cargo clippy -p curbcut-capture --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
