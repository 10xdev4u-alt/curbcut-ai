# curbcut-a11y

Accessibility tree adapters and the unified element model.

## Status

Skeleton. The API is added incrementally; see
[`../../backlog/EPICS.md`](../../backlog/EPICS.md) and
[`../../backlog/ISSUES.md`](../../backlog/ISSUES.md).

## Build and test

```bash
cargo test -p curbcut-a11y
cargo clippy -p curbcut-a11y --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
