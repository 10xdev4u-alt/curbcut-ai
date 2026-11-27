# Confirmation and undo

`crates/curbcut-adapt/src/confirm.rs` states the consequence of a destructive
action, offers undo only when it is possible, and records the outcome. `CC-062`.

The `Act` mode is per-action opt-in (see `docs/ARCHITECTURE.md` §10); a
confirmation is requested only for an action the user has enabled, and it is
requested before anything runs.

## Rules

1. **The consequence is stated.** `Confirmation::consequence` is the
   plain-language sentence the user reads before anything runs. It describes
   what will happen — "The profile and its saved layouts are removed" — never
   what is on screen.
2. **Undo is offered only when possible.** `Confirmation::offers_undo` is the
   single source of truth. `ActionLog::undo_last` refuses to undo an action
   whose confirmation was not undoable, so the interface and the log cannot
   disagree.
3. **The outcome is recorded.** Every confirmation is appended to an
   `ActionLog` with its `Outcome` — `Applied`, `Dismissed`, or `Undone` — and the
   `RuleId` that authorised it, so the user can see what happened.

## Types

- `ActionId` identifies a destructive action. It is a lowercased slug.
- `Confirmation` carries the action, the plain-language consequence, and
  whether the action is undoable. `offers_undo` is the same flag.
- `Decision` is `Confirmed` or `Dismissed`.
- `Outcome` is `Applied`, `Dismissed`, or `Undone`.
- `ActionRecord` is one log entry: the action, the rule, the outcome, and
  whether the action was undoable. `is_undone` is derived from the outcome.
- `ActionLog` is append-only. `record` appends; `undo_last` marks the most
  recent applied, undoable, not-yet-undone action as `Undone` and returns its
  id; `can_undo` reports whether that would do anything; `applied_count` and
  `undone_count` summarise the log; `clear` empties it.

## Undo semantics

`undo_last` scans from the most recent record backwards and skips anything that
is not both applied and undoable. A dismissed action is never undone. A
permanent action is never undone, even if it was confirmed. An action already
undone is not undone twice. When nothing is left, `undo_last` returns `None`.

## Privacy

The log holds action ids, rule references, and outcomes. It never holds screen
content, OCR text, or accessibility strings. The consequence is a sentence the
product wrote, not a transcription of the screen. Retention is `session`: the
log lives in RAM and `clear` empties it; there is no timestamp or expiry. See
[DATA_INVENTORY.md](DATA_INVENTORY.md) §Retention decisions. The workspace
canary scan covers this crate.

## Open work and tracking

Evidence checklist for #216 (all three items are required before any implementation claim):

1. **Production caller.** A named production caller that records action outcomes; a standalone ActionLog or purge with no caller is not evidence.
   Production module: none named today.
2. **Declared window and rollback policy.** A per-class window in DATA_INVENTORY.md plus a rollback-safe clock or day-tick owner; a record count is not a day count.
3. **Retention test and non-blocking purge.** A test fails when a record outlives the window, and the purge is bounded off the UI path.

No production caller constructs an ActionLog today; only the unit tests do. Diagnostic logs have no disk sink (#93), so there is no file to purge. Until all three items exist, DC-20/DC-21/DC-26 stay as documented and no implementation claim is allowed.

- #216 tracks the day-based purge; update this document, the inventory, the threat model, and scripts/validate_retention.py in the same change.
