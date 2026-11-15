# Global shortcuts

The global shortcut catalog lives in `crates/curbcut-input/src/shortcuts.rs`.
`ShortcutAction` lists every global action, `ShortcutCatalog` holds the
bindings, and the platform layer maps a pressed chord to an action without
having to know the defaults. `CC-081`.

## Rules

1. **Defaults do not conflict.** The default catalog has no two actions on the
   same chord, and every default chord is valid.
2. **A chord is a set, not a sequence.** `Ctrl+Shift+D` and `Shift+Ctrl+D` are
   the same chord, so lookup is order-independent and case-insensitive.
3. **A taken chord is rejected.** `ShortcutCatalog::rebind` refuses a chord
   another action already uses, so a conflict can only exist if it is
   introduced directly.

## Defaults

| Action | Default chord | Purpose |
| --- | --- | --- |
| `panic` | `ctrl+shift+esc` | Hide the overlay, stop capture, mute audio, and restore the app. |
| `toggle_dim` | `ctrl+shift+d` | Turn dimming on or off. |
| `toggle_grid` | `ctrl+shift+g` | Turn the icon grid on or off. |
| `toggle_motion` | `ctrl+shift+m` | Turn reduced motion on or off. |
| `toggle_ruler` | `ctrl+shift+r` | Turn the reading ruler on or off. |
| `toggle_read_aloud` | `ctrl+shift+a` | Turn read-aloud on or off. |
| `read_from_here` | `ctrl+shift+enter` | Read the focused text from here. |
| `next_tile` | `ctrl+shift+right` | Move to the next tile in the grid. |
| `previous_tile` | `ctrl+shift+left` | Move to the previous tile in the grid. |
| `activate` | `ctrl+shift+space` | Activate the focused tile. |
| `profiles` | `ctrl+shift+p` | Open the profile picker. |
| `inbox` | `ctrl+shift+i` | Open the interruption inbox. |

The panic key is the one shortcut the product treats as always available: it
hides the overlay, stops capture, mutes audio, and restores the app. The
profile's `panic_key` setting is the persisted form of the `panic` binding.

## Lookup and rebinding

- `ShortcutCatalog::defaults` builds the catalog above.
- `ShortcutCatalog::action_for(&Shortcut)` returns the action a pressed chord
  triggers, comparing chords as sets.
- `ShortcutCatalog::rebind(action, keys)` changes one binding. It returns
  `ShortcutError::Invalid` for an empty chord or an unknown key, and
  `ShortcutError::Conflict` when another action already uses the chord. The
  catalog is unchanged on error.
- `ShortcutCatalog::conflicts` lists every pair of actions that share a chord;
  the default catalog has none.
- `ShortcutCatalog::is_default(action)` reports whether an action still has its
  default chord.

`curbcut_core::config::is_known_key` is public so the catalog and the
configuration validator agree on which key names exist: the modifiers, the
named keys, a single ASCII alphanumeric character, and `f1`..`f12`.

## Privacy

The catalog holds key names and action ids only. It never sees screen content,
OCR text, or accessibility strings, and it never logs a key chord. The
workspace canary scan covers this crate.
