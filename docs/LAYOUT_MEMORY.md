# Per-app layout memory

`crates/curbcut-store/src/layout_memory.rs` remembers the simplification a
user chose for one window of one app, and resets it on request. `CC-087`.

## Scope

A **layout** is a content-free choice: a reading level, a simplification flag,
and a font scale. It is never a label, OCR text, or an accessibility string.
The memory stores the choice and the key it was made under, nothing else.

The key is a `LayoutKey`:

- `user` — a `UserId`, the local user. There is no account system, so the
  caller supplies the local user id.
- `app` — an `AppId` derived from the OS;
- `window` — a `WindowId`.

## Rules

1. **Layouts survive app updates.** The app's version is not part of the key.
   `note_app_update(app)` records that the app was updated and returns how many
   of that app's layouts survived — always all of them. The update count is
   kept so the fact is observable, but it never invalidates a layout.
2. **Memory is per user.** The user is in the key, so two users can hold
   different layouts for the same app and window. The user is a `UserId`, not a
   profile: a person who switches between the `read` and `calm` presets keeps
   one memory, because the presets are not users.
3. **Reset is scoped.** `reset_app(user, app)` drops one user's layouts for one
   app across every window. `reset_user(user)` drops one user's layouts for
   every app. `reset_all()` drops every layout and every recorded update. Each
   returns how many layouts it dropped, so a caller can log a count.
4. **A snapshot is versioned.** `LayoutSnapshot::version` is the schema version.
   `LayoutSnapshotV1` migrates to the current schema; `LayoutSnapshot::migrate`
   refuses a schema newer than this build.

## Migration

Schema v1 stored a reading level and a simplification flag, with no font
scale. The migration performed by `LayoutSnapshotV1::migrate` upgrades every
entry and fills the absent `font_scale` with `DEFAULT_FONT_SCALE` (100). It is
total: it cannot fail, because the only change is a field with a default.

`LayoutSnapshot::migrate` validates a snapshot that is already in the current
shape:

- the current version is returned unchanged;
- a newer version returns `MigrationError::TooNew`, because this build cannot
  know what the newer fields mean;
- any other older version returns `MigrationError::Unsupported`.

`LayoutMemory::from_snapshot` migrates on load, so a snapshot from the future
is refused rather than silently misread.

## API

- `LayoutMemory::{new, from_snapshot, from_v1, snapshot}`.
- `LayoutMemory::{remember, recall, forget}`.
- `LayoutMemory::{reset_app, reset_user, reset_all}`.
- `LayoutMemory::{note_app_update, generation}`.
- `LayoutMemory::{len, is_empty, apps, users}`.
- `LayoutChoice::{new, default}` and `LayoutChoiceV1::{new, upgrade}`.
- `LayoutSnapshot::{new, version, entries, from_v1, migrate}`.
- `LayoutSnapshotV1::{new, migrate}`.
- `MigrationError::{TooNew, Unsupported, found}`.
- `CURRENT_VERSION`, `DEFAULT_READING_LEVEL`, `DEFAULT_FONT_SCALE`.

## Privacy

`LayoutChoice` holds two numbers and a boolean. `LayoutKey` holds a user
number, an app slug, and a window number. `Debug` is derived on both containers
and on `LayoutMemory`, so it prints slugs and counts, never screen content. A
caller that stored content in a choice would print it, so the choice must stay
content-free; the workspace canary scan covers this crate. Deletion for DC-14
is described in [`docs/DATA_INVENTORY.md`](DATA_INVENTORY.md) §Until-deleted
classes. The CLI `forget-layouts` command calls `reset_all` on a process-local
memory built from stdin; it cannot reach a running app's RAM and no store file
exists today.
