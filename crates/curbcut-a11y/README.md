# curbcut-a11y

Accessibility tree adapters and the unified element model.

## Status

The platform role and state mapping is implemented, and the a11y +
vision merge: candidates are matched by IoU and role compatibility,
taken greedily in deterministic order, and each merged property keeps
its provenance. A disagreement becomes a content-free conflict, an
unmatched a11y element stays semantic-only, and an unmatched vision
detection becomes a vision-only node. See
[`../../docs/MERGE.md`](../../docs/MERGE.md).

## Build and test

```bash
cargo test -p curbcut-a11y
cargo clippy -p curbcut-a11y --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
