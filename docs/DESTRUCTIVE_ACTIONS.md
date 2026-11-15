# Destructive-action preview and confirmation

`crates/curbcut-understand/src/actions.rs` decides whether a request is
destructive and shows a plain preview before anything is performed. `CC-050`.

## Rules

1. **The list is data, versioned, and auditable.** `ActionCatalog::version`
   reports `CATALOG_VERSION`, and `ActionCatalog::actions` exposes the whole
   list. The shipped set covers delete, permanent delete, overwrite, send,
   purchase, cancel subscription, revoke, format, publish, transfer, and end
   session.
2. **A preview is shown for every listed action.** `ActionCatalog::preview`
   returns an `ActionPreview` with a plain sentence naming what will happen and
   whether it can be undone. `ActionCatalog::detect` is the only way to reach
   it.
3. **No action is performed without confirmation.** The catalog has no perform
   method. `ActionCatalog::confirm` returns `Confirmation::Required` with the
   preview, and `Confirmation::can_proceed` is false until the user accepts it
   through another path.

## Kinds

`DestructiveKind::{Delete, Overwrite, Send, Purchase, Terminate, Revoke, Format,
Publish}` with `as_str`, `label`, and a fixed `ALL` order.

## Actions

`DestructiveAction::new(id, kind, phrase, consequence, reversible, triggers)`.

- `phrase` — what the action does, in plain words: "delete the selected item".
- `consequence` — what happens as a result: "It is removed from this device."
- `reversible` — whether the user can undo it.
- `triggers` — exact words and phrases that detect the request, including
  inflected forms.

`DestructiveAction::summary` builds the preview sentence: "This will {phrase}.
{consequence}" followed by "You can undo this." or "This cannot be undone."

## Detection

`detect` lowercases the text and matches every trigger. A single-word trigger
sits on word boundaries, so `send` matches "send the message" but not "sender".
A multi-word trigger is a phrase and matches as a substring, so "permanently
delete" matches regardless of surrounding words.

When several triggers match, the longest one wins, so "Permanently delete the
folder." is reported as `permanent-delete` rather than `delete`. Ties break on
catalog order, which keeps the result deterministic.

## API

- `ActionCatalog::new()` / `empty()`, `version`, `actions`, `len`, `is_empty`,
  `find`, `add`, `remove`.
- `detect`, `preview`, `requires_confirmation`, `confirm`.
- `ActionPreview::{action_id, kind, is_reversible, summary}`.
- `Confirmation::{is_required, preview, can_proceed}`.

## Behavior

`confirm` returns `NotRequired` when `detect` finds nothing, and
`Required(preview)` otherwise. A caller that receives `Required` must show the
summary and ask the user; the catalog offers no way to act without that step.

## Privacy

The catalog stores no request text. A preview carries the action id, the kind,
the reversibility, and a summary built from static strings. The workspace
canary scan covers this crate.
