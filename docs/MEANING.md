# Meaning-preservation checker

`crates/curbcut-understand/src/meaning.rs` rejects a simplification that drops
a named entity, a number, a negation, or a comparative. `CC-046`.

## Rules

1. **Recall first.** A missing entity, number, negation, or comparative is a
   rejection, even when the sentence still reads well. The checker would rather
   fall back to the original than show a rewrite that lost a fact.
2. **Comparison is by multiset.** Repeated facts are counted, so "5 items and 5
   more" cannot be reduced to "5 items" and still pass.
3. **Rejection is counted, not quoted.** `MeaningReport` holds counts in four
   categories; `RejectionLog` accumulates them. No token from the screen is
   stored or logged.
4. **The original is the fallback.** `MeaningChecker::rewrite` returns the
   candidate only when `MeaningDecision::Preserved`; otherwise it returns the
   original.

## Categories

| Reason | Extracted by | Examples |
| --- | --- | --- |
| `MissingEntity` | `extract_entities` | `Apple Inc.`, `Bank of America`, `White House` |
| `MissingNumber` | `extract_numbers` | `1,200`, `3.50`, `50%`, `9:30`, `-5` |
| `MissingNegation` | `extract_negations` | `not`, `never`, `neither`, `cannot`, `can't`, `no one`, `without` |
| `MissingComparative` | `extract_comparatives` | `more`, `less`, `than`, `higher`, `lower`, `before`, `most`, `half` |

`RejectionReason::ALL` fixes the reporting order, and `label`/`plural` give the
words used in a summary.

## Entities

`extract_entities` finds runs of capitalized words and trims common words from
both ends. A lowercase connector (`of`, `the`, `and`, `for`, `de`, `van`, `von`,
`del`, `la`, `le`, `du`) continues a run when the next word is capitalized, so
`Bank of America` is one entity. A run made only of common words (`The`, `We`,
`You`, `Do`, `Please`, `New`, `First`, `Same`, `Free`, ...) is not an entity.

## Numbers

`extract_numbers` scans for digit runs with an optional leading `-`, commas,
underscores, decimal points, and a trailing `%`. `normalize_number` removes
`,`, `_`, and `%` and trims trailing zeros after a decimal point, so `1,200`,
`1200`, `1200.00`, and `1_200` all compare equal, and `0.00` equals `0`.

## Negations and comparatives

Both are matched case-insensitively on word boundaries, so `not` matches
"do not" but not "note", and `more` does not match "moreover". A phrase such as
`no one` matches as a phrase.

## API

- `MeaningChecker::{new, check, report, recall, rewrite}`.
- `MeaningDecision::{is_preserved, is_rejected, report, record}`.
- `MeaningReport::{original, simplified, missing, reasons, recall, describe}`.
- `CategoryCounts::{get, set, total, is_empty, reasons}`.
- `RejectionLog::{record, record_report, count, total, is_empty, merge, counts}`.
- `MeaningSimplification::{text, is_original}`.
- `extract_entities`, `extract_numbers`, `extract_negations`,
  `extract_comparatives`.
- `preserved_pairs()`, `adversarial_pairs()` — fixture sets.

## Behavior

`check` extracts all four categories from both texts, counts the original
tokens with no counterpart in the candidate, and returns `Preserved` when
nothing is missing. Otherwise it returns `Rejected` with a report holding the
original counts, the candidate counts, and the missing counts.

`recall` is `1 - missing / original`, or `1.0` when the original has no facts.
`rewrite` returns `Original` for an empty candidate or a rejection.

## Privacy

The report and the log hold counts only. `MeaningReport::describe` is a fixed
sentence built from reason labels and counts, so a rejection can be logged
without quoting the screen. The workspace canary scan covers this crate.
