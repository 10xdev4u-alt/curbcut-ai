# curbcut-a11y

Accessibility tree adapters and the unified element model.

## Status

The platform role and state mapping is implemented, and the a11y +
vision merge: candidates are matched by IoU and role compatibility,
taken greedily in deterministic order, and each merged property keeps
its provenance. A disagreement becomes a content-free conflict, an
unmatched a11y element stays semantic-only, and an unmatched vision
detection becomes a vision-only node, and stability tracking assigns
StableIds across frames with median scroll compensation and a
Hungarian assignment. The write guard makes `WriteGuard::request`
the only path to another app: it requires a valid `WriteA11y`
token, denies and skips the sink otherwise, and records every
attempt. The differential test measures the reading order and the
mapped roles against a recorded screen-reader baseline and publishes
the discrepancy count, so a merge change lowers it or fails. See
[`../../docs/MERGE.md`](../../docs/MERGE.md),
[`../../docs/STABILITY.md`](../../docs/STABILITY.md),
[`../../docs/WRITE_GUARD.md`](../../docs/WRITE_GUARD.md), and
[`../../docs/DIFFERENTIAL_TESTS.md`](../../docs/DIFFERENTIAL_TESTS.md).

## Build and test

```bash
cargo test -p curbcut-a11y
cargo clippy -p curbcut-a11y --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
