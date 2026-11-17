# Keyboard focus order

`crates/curbcut-input/src/focus_order.rs` is the tab cycle over the overlay's
controls. `CC-070`.

## Rules

1. **The cycle is logical.** Controls are visited in the order given, only
   enabled controls are visited, and `advance` / `back` wrap around.
2. **There is no trap.** A trapping control keeps Tab on itself, so `escape` is
   the way out. `validate` rejects an order where every enabled control traps
   focus, because then focus can never leave.
3. **Shortcuts are discoverable.** `shortcuts` lists every focusable's shortcut
   in cycle order, so the UI can show it.

Arrow keys move spatially with `nearest`, which picks the closest enabled
control in a direction and breaks ties by id.

## Types

- `FocusableId` — a `u64` newtype with `get`.
- `Direction` — `Up`, `Down`, `Left`, `Right`.
- `Focusable` — an id, a `Rect`, an `enabled` flag, a `trap` flag, and an
  optional `ShortcutAction`. `new(id, rect)` is enabled, non-trapping, and has
  no shortcut; `with_enabled`, `with_trap`, and `with_shortcut` adjust it.
- `FocusOrderError` — `NoEnabled`, `DuplicateId`, `AllTrapped`.

## Methods

| Method | Meaning |
| --- | --- |
| `new(items)` | Creates an order focused on the first enabled control. |
| `current` / `current_id` / `is_trapped` | The focused control. |
| `focus(id)` / `focus_index(i)` | Focuses an enabled control; a disabled one is refused. |
| `advance` / `back` | Moves through enabled controls, wrapping. |
| `first` / `last` | Focuses the first or last enabled control. |
| `escape` | Leaves a trapping control for the next enabled one, wrapping. |
| `nearest(direction)` | The closest enabled control in a direction. |
| `shortcuts` | Every focusable's shortcut in cycle order. |
| `validate` | Checks the order. |

`nearest` scores a candidate by its distance along the direction plus twice its
perpendicular offset, and picks the lowest score; equal scores go to the
smaller id. A candidate in the wrong direction is ignored.

## Privacy

The order reads ids, rectangles, and shortcut actions. It never reads a label or
screen content, so a focus dump cannot leak what is on screen.

## API

- `FocusableId`, `Direction`, `Focusable`, `FocusOrderError`, `FocusOrder`.
