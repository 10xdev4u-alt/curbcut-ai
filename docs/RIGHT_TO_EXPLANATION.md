# Right to explanation

`crates/curbcut-adapt/src/report.rs` answers one question from a user: *what
has CurbCut been doing?* It builds a plain-language report of every action, why
it happened, and the original value it replaced. `CC-122`.

## Inputs

`Report::from_sources` takes three things:

- the **action log** (`confirm::ActionLog`), which records what the user
  confirmed or dismissed;
- the **provenance panel** (`provenance::ProvenancePanel`), which records the
  rule, the setting, and the original value for each change;
- the **rationales** (`core::types::Rationale`), which carry the rule prose.

The two logs are independent, so they are paired by action id in the order each
recorded them: the first `dim` in the action log is the first `dim` in the
provenance panel. An action that appears in only one of them is still reported,
with the missing half left blank, so the report never silently drops an action.
The first rationale for a rule wins, so the report's reasoning cannot change
because a later rule offered different prose.

`generated_at_ms` is supplied by the caller; the module has no clock.

## Every action links to its rationale

An entry with no rationale renders `Why: no reason was recorded.` rather than
omitting the line, and `Report::missing_rationale` names every such action, so
`Report::is_explained` is a checkable claim. An empty or whitespace-only
rationale counts as missing.

## Readability is measured

`Report::grade_level` runs the Flesch-Kincaid grade over the report body — the
header and the entries, excluding the footer that quotes the grade — and
`Report::is_readable_at(grade)` compares it with a target. The issue's
"readable at a grade-6 level" is therefore a measured outcome: the report says
its own grade in the footer, and a test asserts it against the text.

## Redaction

The export is redacted of other apps' content:

- A content-derived original is rendered as `content, N bytes (identity H)`.
  The text was never retained, so there is nothing to leak.
- A setting original is rendered as its value, because a setting is user-chosen,
  not another app's content.
- An action id is a fixed action name, never a label from the screen.
- Rule prose is rule prose, never screen content.

`OriginalValue::describe` remains the log-safe renderer: it prints only the
value's type. `OriginalValue::render` is the user-facing one, and the report uses
it.

## Exports

`to_plain_text` is the user export. `to_markdown` is the same content with light
markup, for the UI and for filing. Both are derived from the same entries, so
they cannot disagree.

## Rules

1. **Every action is explained or flagged.**
2. **The report is readable, and says its own grade.**
3. **The export carries no other app's content.**

## Privacy

`Debug` on a report prints the entries, and through them a setting value.
Settings are user-chosen, and content is already reduced to a length and a hash,
so no screen content can reach a dump. The report is a user-facing artifact, not
a log sink; it should never be written to one.
