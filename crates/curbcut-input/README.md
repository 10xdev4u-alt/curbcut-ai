# curbcut-input

Global shortcuts, hover/click routing, and switch access.

## Status

The global shortcut catalog is implemented: `ShortcutAction` lists
twelve actions, `ShortcutCatalog` holds the bindings, detects conflicts,
and rebinds with a taken-chord check. The pointer tracker is
implemented: `HoverTracker` classifies movement from samples so a hover
is dwell, not presence. Click-through routing is implemented:
`ClickRouter` consumes or hands off each click, never both, and a
handoff owes focus until it returns. The keyboard focus order is
implemented: `FocusOrder` visits enabled controls in order, wraps, keeps
a trapping control until `escape`, lists every shortcut for the UI, and
moves spatially with the arrow keys. Switch access is implemented:
`SwitchAccess` drives the overlay from one switch with auto scan, step
scan, and a configurable dwell. Action consent is implemented:
`ConsentGate` requires a per-session, per-action consent, states the
consequence before the choice, scopes a receipt to one action or to the
session, and logs action class and target role only. See
[`../../docs/SHORTCUTS.md`](../../docs/SHORTCUTS.md),
[`../../docs/HOVER.md`](../../docs/HOVER.md),
[`../../docs/CLICKS.md`](../../docs/CLICKS.md),
[`../../docs/FOCUS_ORDER.md`](../../docs/FOCUS_ORDER.md),
[`../../docs/SWITCH_ACCESS.md`](../../docs/SWITCH_ACCESS.md), and
[`../../docs/CONSENT.md`](../../docs/CONSENT.md).

## Build and test

```bash
cargo test -p curbcut-input
cargo clippy -p curbcut-input --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
