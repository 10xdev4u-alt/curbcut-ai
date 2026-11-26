# Interruption inbox

`crates/curbcut-adapt/src/inbox.rs` files detected interruptions into a quiet,
review-on-demand list. `CC-060`.

The detection layer reports a toast, banner, popup, cookie notice, or paywall.
The inbox keeps it until the user acts. It is the adaptation engine's side of
`profile.focus.interruptions = "inbox"`.

## Rules

1. **Nothing is auto-dismissed.** The inbox has no timer, expiry, or
   auto-accept. Filing is the caller's decision; removal is the user's through
   `remove` or `clear`.
2. **Batching is by app and kind.** `batches` groups entries by the pair
   `(app, kind)` and sorts the groups by app then kind, so the same set of
   entries always produces the same groups. Entries inside a group keep filing
   order.
3. **The panel is review-on-demand.** `open`, `close`, and `toggle` drive one
   flag. The module never opens itself.
4. **The list is keyboard navigable.** `move_next` and `move_prev` move one
   cursor over the flat list and clamp at the ends, so a held key cannot run
   off the list. `move_to` jumps to a filed id. Removing an entry before the
   cursor shifts the cursor with it, so the entry under it stays the same.

## API

- `Inbox` — `new`, `open`, `close`, `toggle`, `is_open`, `file`, `file_all`,
  `entries`, `len`, `is_empty`, `get`, `batches`, `current`, `current_id`,
  `move_next`, `move_prev`, `move_to`, `remove`, `clear`.
- `InboxEntry` — `id`, `app`, `kind`, `rect`, `dismissible`.
- `InboxBatch` — `app`, `kind`, `items`, `len`, `is_empty`.
- `InboxId` — a `Copy` id newtype with `get`.

## Filing

`file(app, interruption)` stores the kind, rectangle, and dismissibility flag
under the app that produced it and returns the new id. `file_all` files a batch
from one app and returns the ids in order. Filing the same interruption twice
files it twice; coalescing by stable id is the caller's decision, not the
inbox's.

## Privacy

An entry holds an app slug, a kind, a rectangle, and a flag. It holds no label
or text, so an inbox dump cannot leak what was on screen. The CLI `forget-inbox`
command calls `Inbox::clear` on a process-local inbox built from stdin; it
reaches no running app and no store file. Deletion for DC-24 is described in
[`docs/DATA_INVENTORY.md`](DATA_INVENTORY.md) §Until-deleted
classes.
