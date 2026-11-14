# curbcut-adapt

Profiles, rules engine, layout solver, and plan cache.

## Status

The profile graph is implemented: a versioned settings graph with a
baseline, atomic multi-setting changes, live-preview sinks, and a
40-key setting registry. See
[`../../docs/PROFILE_GRAPH.md`](../../docs/PROFILE_GRAPH.md).

## Build and test

```bash
cargo test -p curbcut-adapt
cargo clippy -p curbcut-adapt --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
