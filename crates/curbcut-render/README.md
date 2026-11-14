# curbcut-render

Overlay plan types, hit-testing, and rendering contracts.

## Status

Skeleton. The API is added incrementally; see
[`../backlog/EPICS.md`](../backlog/EPICS.md) and
[`../backlog/ISSUES.md`](../backlog/ISSUES.md).

## Build and test

```bash
cargo test -p curbcut-render
cargo clippy -p curbcut-render --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
