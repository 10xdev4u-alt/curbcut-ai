# curbcut-understand

Classification, chunking, simplification, summarization, and reading level.

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
medical, and legal text and always returns the original. A versioned
destructive-action catalog returns a plain preview and requires
confirmation before anything is performed. The meaning-preservation
checker rejects a rewrite that drops a named entity, a number, a
negation, or a comparative, and logs rejections as counts only. Text
chunking splits plain text into headings, list items, tables, sentences,
and clauses as half-open byte ranges, keeps a table whole, and never
splits mid-word. Interruption detection reports the transient overlays
in a merged graph — toast, banner, popup, cookie notice, or paywall —
with a rectangle and a dismissibility flag, and reads labels in memory
only. Icon semantics combines an icon's accessible label, its caption,
and its neighbours into a plain what-this-does label of at most eight
words, and marks a guess as uncertain. Extractive summarization ranks
sentences by mean term frequency, skips near-duplicates, and emits key
points in reading order with a byte range and chunk id per point, while
ROUGE-1 and ROUGE-L recall are reported rather than gated. See
[`../../docs/RULES.md`](../../docs/RULES.md),
[`../../docs/READING_LEVEL.md`](../../docs/READING_LEVEL.md),
[`../../docs/JARGON.md`](../../docs/JARGON.md),
[`../../docs/TEXT_CHUNKING.md`](../../docs/TEXT_CHUNKING.md),
[`../../docs/INTERRUPTIONS.md`](../../docs/INTERRUPTIONS.md),
[`../../docs/ICON_SEMANTICS.md`](../../docs/ICON_SEMANTICS.md),
[`../../docs/SUMMARIZE.md`](../../docs/SUMMARIZE.md),
[`../../docs/NORMALIZE.md`](../../docs/NORMALIZE.md),
[`../../docs/SENSITIVE_CONTENT.md`](../../docs/SENSITIVE_CONTENT.md), and
[`../../docs/DESTRUCTIVE_ACTIONS.md`](../../docs/DESTRUCTIVE_ACTIONS.md), and
[`../../docs/MEANING.md`](../../docs/MEANING.md).

## Build and test

```bash
cargo test -p curbcut-understand
cargo clippy -p curbcut-understand --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
