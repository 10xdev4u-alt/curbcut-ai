# curbcut-input

Global shortcuts, hover/click routing, and switch access.

## Status

The global shortcut catalog is implemented: `ShortcutAction` lists
twelve actions, `ShortcutCatalog` holds the bindings, detects conflicts,
and rebinds with a taken-chord check. The pointer tracker is
implemented: `HoverTracker` classifies movement from samples so a hover
is dwell, not presence. See
[`../../docs/SHORTCUTS.md`](../../docs/SHORTCUTS.md) and
[`../../docs/HOVER.md`](../../docs/HOVER.md).

## Build and test

```bash
cargo test -p curbcut-input
cargo clippy -p curbcut-input --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
