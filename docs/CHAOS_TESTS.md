# Chaos testing: failure injection

Chaos testing drives a component through injected failures and checks that the
failure is survivable, announced, and recovered. It complements the property
suite: a property proves an invariant holds for every input, while a chaos run
proves the system behaves when an input *cannot* be processed.

The harness is deterministic and dependency-free. It lives in
`crates/curbcut-core/src/chaos.rs`; the suites are in
`crates/curbcut-core/tests/chaos.rs`.

## The three invariants

1. **No panic escapes.** Every injection runs inside
   [`guard`](curbcut_core::chaos::guard). `guard` replaces the process panic
   hook for the duration of the operation, so the default hook cannot print
   the payload to stderr before it is caught, then turns the panic into
   `Err(Caught)` and drops the payload unread. A panic message can quote its
   input, so it must never reach a log.
2. **A fault is announced.** After injection the component reports a
   [`Degradation`](curbcut_core::chaos::Degradation) carrying a static reason
   code. A fault with no announcement fails the report.
3. **Recovery is automatic.** `Faulty::recover` restores the baseline, and the
   report records whether it did.

A run is clean only when all three hold:

```rust
let report = curbcut_core::chaos::run(&mut gate, &script);
assert!(report.is_clean(), "{report}");
```

## The faults

| Fault | What it models | Surface |
| --- | --- | --- |
| `PermissionRevoked` | An OS permission prompt dismissed while the app runs | `Session::revoke`, which invalidates every token |
| `CacheCorrupt` | A torn write or stale buffer | `ChecksumCache`, whose reads return `None` on a checksum mismatch |
| `HugeScreen` | A 5K-class display, or coordinates near `i32` extremes | `ScreenBudget`, backed by `Rect` |
| `RemoteDesktop` | Capture over RDP, where frames may be blocked | `CaptureLink` |

`Script` schedules faults at step indices; `Script::random` draws one from a
seed, so a failing ordering replays exactly.

## The components

`Faulty` is a trait, so each crate plugs in its own surface. The core module
ships four implementations:

- [`PermissionGate`](curbcut_core::chaos::PermissionGate) — grants a capability,
  revokes it mid-session, then re-grants on recovery.
- [`ChecksumCache`](curbcut_core::chaos::ChecksumCache) — a deliberately
  corruptible byte cache. This is an injection surface, not production storage.
- [`ScreenBudget`](curbcut_core::chaos::ScreenBudget) — geometry at extreme
  sizes.
- [`CaptureLink`](curbcut_core::chaos::CaptureLink) — local, remote, or
  unavailable capture.

To fault a production type in another crate, implement `Faulty` for a newtype
around it and pass it to `run`. `accepts` lets one script drive surfaces with
different responsibilities.

## A real bug this suite caught

`Rect::right` and `Rect::bottom` computed `x + (width as i32)`. A width above
`i32::MAX` wrapped negative, so a rectangle could report a right edge left of
its origin. The edges now compute in `i64` and clamp, and `intersection` /
`union` clamp their dimensions the same way. Regression tests live in
`crates/curbcut-core/src/types.rs`.

## Privacy

Reason codes are static strings. A degradation names a condition, never a
screen value, OCR text, or accessibility string. `Caught` carries no payload.

## Running

```bash
cargo test -p curbcut-core --test chaos
```

CI runs the same command in the `chaos` job with `CURBCUT_PROP_CASES=5000`,
under a job timeout. The ordinary `rust` job also runs the file as part of
`cargo test --workspace`.
