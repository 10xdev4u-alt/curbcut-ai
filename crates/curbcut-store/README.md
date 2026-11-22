# curbcut-store

Storage schema, migrations, repositories, key handling, the plan cache, and the sealed understanding cache.

## Status

The plan cache is implemented: a bounded LRU cache of overlay plans
keyed by app, window, and content hash, and opt-in per-app memory for
one remembered choice per app. The bounded LRU lives in `lru` and is
shared with `understanding_cache`, which caches encoded understanding
results keyed by content hash, model version, and profile, and stores
every value as ciphertext from a caller-supplied `Cipher`. See
[`../../docs/PLAN_CACHE.md`](../../docs/PLAN_CACHE.md) and
[`../../docs/UNDERSTANDING_CACHE.md`](../../docs/UNDERSTANDING_CACHE.md).

## Build and test

```bash
cargo test -p curbcut-store
cargo clippy -p curbcut-store --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
