# curbcut-input

Global shortcuts, hover/click routing, and switch access.

## Status

The global shortcut catalog is implemented: `ShortcutAction` lists
twelve actions, `ShortcutCatalog` holds the bindings, detects conflicts,
and rebinds with a taken-chord check. The pointer tracker is
implemented: `HoverTracker` classifies movement from samples so a hover
is dwell, not presence. Click-through routing is implemented:
`ClickRouter` consumes or hands off each click, never both, and a
handoff owes focus until it returns. See
[`../../docs/SHORTCUTS.md`](../../docs/SHORTCUTS.md),
[`../../docs/HOVER.md`](../../docs/HOVER.md), and
[`../../docs/CLICKS.md`](../../docs/CLICKS.md).

## Build and test

```bash
cargo test -p curbcut-input
cargo clippy -p curbcut-input --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
