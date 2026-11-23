# curbcut-store

Storage schema, migrations, repositories, key handling, the plan cache, the sealed understanding cache, per-app layout memory, and learned simplifications.

## Status

The plan cache is implemented: a bounded LRU cache of overlay plans
keyed by app, window, and content hash, and opt-in per-app memory for
one remembered choice per app. The bounded LRU lives in `lru` and is
shared with `understanding_cache`, which caches encoded understanding
results keyed by content hash, model version, and profile, and stores
every value as ciphertext from a caller-supplied `Cipher`. Per-user,
per-app, per-window layout memory remembers a content-free choice,
keeps it across an app update, resets per app, per user, or globally,
and migrates a schema-v1 snapshot by filling the absent font scale.
Learned simplifications record a content hash and an accept/reject
verdict, move a simplification level by a bounded step only after a
minimum number of observations and records, keep a bounded undo history,
and report the acceptance rate. Sealed values are held in a zeroizing
`SecureBuffer`, so eviction overwrites them rather than merely
unlinking. See
[`../../docs/PLAN_CACHE.md`](../../docs/PLAN_CACHE.md),
[`../../docs/UNDERSTANDING_CACHE.md`](../../docs/UNDERSTANDING_CACHE.md),
[`../../docs/LAYOUT_MEMORY.md`](../../docs/LAYOUT_MEMORY.md), and
[`../../docs/LEARNED_SIMPLIFICATIONS.md`](../../docs/LEARNED_SIMPLIFICATIONS.md).

## Build and test

```bash
cargo test -p curbcut-store
cargo clippy -p curbcut-store --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
