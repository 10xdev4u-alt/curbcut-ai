# curbcut-app

Tauri app shell and frontend bridge (stub until the toolchain lands).

## Status

Skeleton. The API is added incrementally; see
[`../backlog/EPICS.md`](../backlog/EPICS.md) and
[`../backlog/ISSUES.md`](../backlog/ISSUES.md).

## Build and test

```bash
cargo test -p curbcut-app
cargo clippy -p curbcut-app --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
