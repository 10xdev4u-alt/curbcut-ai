# curbcut-understand

Classification, chunking, simplification, and reading level.

## Status

The deterministic rule layer is implemented: versioned rules, role
refinement, chunking, reading-level comparison, and distraction scoring.
Reading level scores Flesch, Flesch-Kincaid, SMOG, and ARI, flags
non-Latin scripts as unsupported, and caches scores by content hash.
See [`../../docs/RULES.md`](../../docs/RULES.md) and
[`../../docs/READING_LEVEL.md`](../../docs/READING_LEVEL.md).

## Build and test

```bash
cargo test -p curbcut-understand
cargo clippy -p curbcut-understand --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
