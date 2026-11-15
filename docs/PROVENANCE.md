# Provenance panel

`crates/curbcut-adapt/src/provenance.rs` answers one question per change: *why
was this made?* Every entry links an action to the rule that authorised it and
to the value it replaced. `CC-065`.

## Rules

1. **Every action links to a rule.** `ProvenancePanel::record` takes a `RuleId`
   and there is no way to record a change without one. A row without a rule is
   not representable.
2. **The original is retained only as far as it is safe.** A setting value is
   kept whole, because settings are user-chosen. Content-derived text is kept
   as a length and a non-cryptographic identity hash, never as text.
3. **The panel is log-safe.** `OriginalValue::describe` renders a content-free
   summary, so a debug dump cannot leak what was on screen.

## Types

- `ProvenanceId` — identifies one entry; `get` returns the numeric id.
- `OriginalValue` — `Setting(SettingValue)`, `Content { len, hash }`, or `None`.
  `OriginalValue::content` hashes text without retaining it; `describe` is the
  log-safe summary.
- `ProvenanceEntry` — `id`, `action`, `rule`, `setting`, `original`,
  `changed_at_ms`.
- `ProvenancePanel` — `new`, `len`, `is_empty`, `entries`, `record`, `get`,
  `original`, `for_action`, `for_rule`, `clear`.

`entries` is oldest first. `get` and `original` are the one-click lookup: the
panel shows the rule and the original for an id. `for_action` and `for_rule`
filter without copying.

## Privacy

The panel stores action ids, rule ids, setting keys, timestamps, and either a
setting value or a content hash. It never stores screen content, OCR text, or
accessibility strings. `describe` is safe to log; the content hash is an
identity check, not a security boundary. The workspace canary scan covers this
crate.
