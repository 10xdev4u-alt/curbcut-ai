# curbcut-understand

Classification, chunking, simplification, and reading level.

## Status

The deterministic rule layer is implemented: versioned rules, role
refinement, chunking, reading-level comparison, and distraction scoring.
See [`../../docs/RULES.md`](../../docs/RULES.md).

## Build and test

```bash
cargo test -p curbcut-understand
cargo clippy -p curbcut-understand --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
