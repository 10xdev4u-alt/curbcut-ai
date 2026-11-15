# Sensitive-content guard

`crates/curbcut-understand/src/sensitive.rs` refuses to auto-simplify
financial, medical, and legal text. `CC-051`.

## Rules

1. **The guard is conservative.** A strong signal triggers on its own. A weak
   signal triggers only when a second weak signal from the same domain also
   matches. Ordinary prose containing one ambiguous word does not trigger.
2. **The original is always visible.** `SensitiveGuard::auto_simplify` returns
   `Simplification::Original` carrying the original text when the guard refuses.
   A caller can never show a rewrite the guard has not cleared.
3. **Nothing is logged.** `SensitiveReport` counts hits and names domains. It
   never carries the text it scanned, and its `Debug` output is safe.
4. **The signal table is data.** `SensitiveGuard::signals` is the auditable
   list. It can be extended, filtered, or cleared without touching the logic.

## Domains

- `SensitiveDomain::Financial` — money, banking, credit, tax.
- `SensitiveDomain::Medical` — health, diagnosis, medication.
- `SensitiveDomain::Legal` — law, courts, contracts, rights.

`SensitiveDomain::ALL` fixes the report order, and `label` gives the
human-readable name used in the warning.

## Signals

`SensitiveSignal::new(marker, domain, strength)` with
`SignalStrength::{Strong, Weak}`.

Strong markers are unambiguous domain words and phrases: `deductible`,
`co-pay`, `out-of-pocket`, `credit score`, `bank account`, `wire transfer`,
`mortgage`, `invoice`, `diagnosis`, `prescription`, `dosage`, `biopsy`,
`side effect`, `oncology`, `lab results`, `pre-existing`, `plaintiff`,
`subpoena`, `statute`, `warrant`, `power of attorney`, `terms of service`,
`attorney`.

Weak markers are ambiguous single words: `bank`, `account`, `charge`, `bill`,
`interest`, `balance`, `statement`, `fee`, `claim`, `coverage`, `provider`,
`policy`, `court`, `state`, `case`, `contract`, `rights`, `legal`. A single
weak marker never triggers; two from the same domain do.

Matching is case-insensitive. A single-word marker sits on word boundaries, so
`bank` matches "river bank" but not "banking". A multi-word marker is a phrase
and matches as a substring, so `power of attorney` matches with any spacing.

## API

- `SensitiveGuard::new(weak_threshold)` — built-in table; the default threshold
  is 2. `empty()` has no signals.
- `scan(text) -> GuardDecision` — `Clear` or `Refused(SensitiveReport)`.
- `report(text) -> Option<SensitiveReport>`, `domains(text)`,
  `is_sensitive(text)`.
- `auto_simplify(original, candidate) -> Simplification` — `Simplified` only
  when both texts are clear and the candidate is non-empty; otherwise
  `Original`.
- `SensitiveReport::{domains, is_sensitive, strong_hits, weak_hits, message}`.
- `GuardDecision::{is_refused, report}`; `Simplification::{text, is_original}`.

## Behavior

`scan` lowercases the text once, then walks the signal table. Strong hits are
counted per domain; weak hits are counted per domain. A domain triggers when it
has a strong hit or at least `weak_threshold` weak hits. Domains are reported in
`SensitiveDomain::ALL` order. `set_weak_threshold(0)` disables weak signals.

`auto_simplify` checks both the original and the candidate, so a sensitive
rewrite cannot slip through a clear original. An empty candidate falls back to
the original.

## Privacy

The guard stores nothing. It returns counts and domain names, and
`SensitiveReport::message` is a fixed sentence naming domains. The workspace
canary scan covers this crate.
