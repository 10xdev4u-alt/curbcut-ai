# curbcut-store

Storage schema, migrations, repositories, key handling, and the plan cache.

## Status

The plan cache is implemented: a bounded LRU cache of overlay plans
keyed by app, window, and content hash, and opt-in per-app memory for
one remembered choice per app. See
[`../../docs/PLAN_CACHE.md`](../../docs/PLAN_CACHE.md).

## Build and test

```bash
cargo test -p curbcut-store
cargo clippy -p curbcut-store --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
