# curbcut-understand

Classification, chunking, simplification, and reading level.

## Status

The deterministic rule layer is implemented: versioned rules, role
refinement, chunking, reading-level comparison, and distraction scoring.
Reading level scores Flesch, Flesch-Kincaid, SMOG, and ARI, flags
non-Latin scripts as unsupported, and caches scores by content hash.
The jargon dictionary carries a source and locale per entry, supports
user overrides, and expands acronyms while flagging unknown ones.
Numbers, dates, and measurements are spoken: money in hundreds,
ambiguous dates rejected, and a locale's unit system honoured.
The sensitive-content guard refuses to auto-simplify financial,
medical, and legal text and always returns the original. See
[`../../docs/RULES.md`](../../docs/RULES.md),
[`../../docs/READING_LEVEL.md`](../../docs/READING_LEVEL.md),
[`../../docs/JARGON.md`](../../docs/JARGON.md),
[`../../docs/NORMALIZE.md`](../../docs/NORMALIZE.md), and
[`../../docs/SENSITIVE_CONTENT.md`](../../docs/SENSITIVE_CONTENT.md).

## Build and test

```bash
cargo test -p curbcut-understand
cargo clippy -p curbcut-understand --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
