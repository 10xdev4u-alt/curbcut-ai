# Property tests and parser fuzzing

A property test states an invariant that must hold for every input, then checks
it against generated inputs. A unit test proves one example; a property test
fails with the input class that breaks the rule.

The workspace builds offline and carries no dependencies, so the harness is a
small seeded generator rather than a property-test crate. It lives in
`crates/curbcut-core/src/prop.rs` and every crate uses it.

## The harness

| Item | Role |
| --- | --- |
| `Rng` | Splitmix64 generator. `Rng::new(seed)` reproduces a stream exactly. |
| `run` | Runs a property over `cases` cases derived from a seed. |
| `run_budgeted` | Same, and stops at a wall-clock budget instead of failing on it. |
| `Failure` | Seed, case index, and a static message. |

Case `n` uses `seed.wrapping_add(n)`, so a failure at case `n` can be replayed on
its own:

```rust
let mut rng = curbcut_core::prop::Rng::new(0x2000 + n);
```

A property returns `Ok(())` when the invariant holds and `Err` with a fixed
description when it breaks. The description never includes a generated value: a
failure message reaches the test log, and the log carries counts and hashes
only.

## Two budgets

Two knobs shape a run, both read from the environment with a default:

- `CURBCUT_PROP_CASES` — how many cases a property test runs.
  Default `2_000`.
- `CURBCUT_FUZZ_BUDGET_MS` — the wall-clock cap for a fuzz run, in
  milliseconds. Default `30_000`.

A missing or unparseable value is the default, never an error. Running out of
budget is not a failure: CI raises the case count and lets the budget stop the
run, so a slow runner does not turn into a red build.

## Where the properties live

| Suite | File | Invariants |
| --- | --- | --- |
| Core properties | `crates/curbcut-core/tests/properties.rs` | A rejected config document never replaces the active config; a valid profile round-trips; booleans re-apply idempotently; redaction is idempotent and removes every canary; hash mode is stable; ids preserve order; digests round-trip; HMAC depends on key and message; reading levels round-trip. |
| Parser fuzz | `crates/curbcut-core/tests/fuzz_parsers.rs` | The TOML parser never panics; an accepted document always validates; a fuzzed canary list still redacts every entry; hex parsing never panics and round-trips; arbitrary bytes never panic the hash layer. |
| Merge properties | `crates/curbcut-a11y/tests/properties.rs` | The report accounts for every input element; merged ids are unique; source flags agree with the counts; merging is deterministic; an empty merge is a no-op. |
| Store properties | `crates/curbcut-store/tests/properties.rs` | A plan cache respects its capacity; LRU order matches a recency model; invalidation leaves no stale hit; the sealed understanding cache round-trips and bounds itself; feedback counts are consistent; clamped adaptation settings are in range; a learned level stays in range; layout memory resets only what it names. |
| Adapt properties | `crates/curbcut-adapt/tests/properties.rs` | A successful setting write leaves a valid graph; a batch is atomic; a reset restores the baseline; a dim plan is exact, non-overlapping, and protected; an out-of-range opacity is rejected. |

## Privacy by construction

The generator's alphabet is lowercase letters, digits, punctuation, and
whitespace. It cannot spell a canary prefix, so a fuzz case can never be
mistaken for a leak. Tests that need a real canary build it from
`Redactor::canaries()` or `curbcut_core::redact::CANARY_PREFIX` at runtime;
the fixture strings are never written into source.

## Running them

```bash
cargo test -p curbcut-core --test properties --test fuzz_parsers
cargo test -p curbcut-a11y -p curbcut-store -p curbcut-adapt --test properties
```

CI runs the same two commands in the `property` job with
`CURBCUT_PROP_CASES=5000` and `CURBCUT_FUZZ_BUDGET_MS=60000`, under a job
timeout. The ordinary `rust` job also runs these files as part of
`cargo test --workspace`.
