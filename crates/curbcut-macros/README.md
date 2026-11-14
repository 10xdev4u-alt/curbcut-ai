# curbcut-macros

Typed event and provenance macros.

## Status

Skeleton. The API is added incrementally; see
[`../backlog/EPICS.md`](../backlog/EPICS.md) and
[`../backlog/ISSUES.md`](../backlog/ISSUES.md).

## Build and test

```bash
cargo test -p curbcut-macros
cargo clippy -p curbcut-macros --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
