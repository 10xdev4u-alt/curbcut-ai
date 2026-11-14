# curbcut-adapt

Profiles, rules engine, layout solver, and plan cache.

## Status

The profile graph and layout solver are implemented: a versioned
settings graph with a baseline, atomic multi-setting changes,
live-preview sinks, and a 40-key setting registry; and a priority-based
tile packer plus a no-clip reflow; and a dimming planner that covers
the screen except the focused control and except-regions. See
[`../../docs/PROFILE_GRAPH.md`](../../docs/PROFILE_GRAPH.md),
[`../../docs/LAYOUT.md`](../../docs/LAYOUT.md), and
[`../../docs/DIMMING.md`](../../docs/DIMMING.md).

## Build and test

```bash
cargo test -p curbcut-adapt
cargo clippy -p curbcut-adapt --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
