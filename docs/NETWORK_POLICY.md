# Network policy and the kill-switch

## Why

CurbCut runs on a screen full of other people's work. A background agent that
dials out — for a model, a profile sync, a telemetry ping — spends the user's
bandwidth, leaks the fact that they are using CurbCut, and can leak more if the
payload is ever wrong. The product is offline first: perception, understanding,
adaptation, rendering, and speech all run on the device.

Two independent gates enforce that, and either one alone is enough to stop a
connection:

- **The compiled gate.** The `net` cargo feature is what puts the optional
  transport in the binary at all. It is opt-in: the crate's default feature set
  is empty, so a plain build has no `transport` module and an optional feature
  has no code path that can reach a socket. `--features net` adds the module;
  `--no-default-features` also leaves it out.
- **The runtime gate.** `KillSwitch` holds a `NetworkPolicy` and an engaged
  flag. It starts engaged, so a fresh switch denies everything even with an
  online policy; `release()` hands the decision to the policy, and `engage()`
  denies everything again.

## The channels

| Feature | String | Without the `net` feature | Purpose |
| --- | --- | --- | --- |
| `ModelDownload` | `model_download` | removed | fetch a model, vocabulary, or voice |
| `UpdateCheck` | `update_check` | kept | check for a newer build |
| `Sync` | `sync` | removed | optional encrypted profile sync |
| `Telemetry` | `telemetry` | removed | opt-in, counts-only telemetry |
| `LicenseCheck` | `license_check` | removed | validate a licence or donation key |

`UpdateCheck` is the one channel every build keeps: it is how a broken install
is repaired. It still consults the switch, so an engaged switch blocks updates
too. `update::dial` takes the feature it is dialling and returns
`NetworkError::Blocked` for anything but `UpdateCheck`, so a denied optional
channel cannot ride the update path.

Crash reports and companion read-aloud are not channels today; see
[`docs/TRANSMISSION_PATHS.md`](TRANSMISSION_PATHS.md).

## The modes

| Mode | String | Allows |
| --- | --- | --- |
| `Offline` | `offline` | nothing |
| `UpdatesOnly` | `updates_only` | `UpdateCheck` |
| `Online` | `online` | every channel, subject to the switch |

`NetworkPolicy` is data. It holds no socket, no clock, and no I/O, so the same
policy always answers the same question and can be stored in the profile graph.
It defaults to offline.

## The gate in use

```rust
use curbcut_core::net::{KillSwitch, NetworkFeature, NetworkPolicy};

// Deny by default: a new switch is engaged.
let mut switch = KillSwitch::new(NetworkPolicy::updates_only());
assert!(!switch.allows(NetworkFeature::Sync));

// The user turns on updates in the profile.
switch.release();
assert!(switch.allows(NetworkFeature::UpdateCheck));
assert!(!switch.allows(NetworkFeature::Sync));

// The panic key engages the switch and blocks everything, updates included.
switch.engage();
assert!(!switch.allows(NetworkFeature::UpdateCheck));
```

`transport::dial` (compiled only with the `net` feature) and `update::dial`
consult the switch at the boundary, so a caller cannot forget the gate. Both
return `NetworkError::Blocked` without dialling when the switch refuses.

## Which build is running

`net::transport_available()` reports whether the optional transport was
compiled in. The test suite asserts both directions:

- with `net`, the optional transport exists and follows the switch;
- without it, no optional transport exists and only the update channel can
  dial, still behind the switch.

Because the crate's default is empty, `cargo tree --workspace -e features`
shows no `net` feature anywhere: a workspace build is offline unless a crate
opts in. `--features net` is the explicit opt-in.

## The socket guard

A decision in a test can be wrong, so the guarantee is also measured at the
syscall layer. `scripts/socket_guard.py` runs a command under `strace` and
fails on any `connect(2)` that is not an `AF_UNIX` socket. Its `--self-test`
connects to a refused loopback port to prove the guard is not vacuous.

The `net_probe` example binds a loopback listener, prints the decision for
every channel, and dials every channel the switch denies. Every dial is
refused, so the probe opens no socket; if a gate regressed, the dial would
reach the listener and the probe would panic.

The `socket-guard` CI job runs the self-test, then guards the probe in both
builds:

```bash
cargo build -p curbcut-core --example net_probe
python3 scripts/socket_guard.py ./target/debug/examples/net_probe
cargo build -p curbcut-core --features net --example net_probe
python3 scripts/socket_guard.py ./target/debug/examples/net_probe
```

The four-platform `rust` job also runs clippy and the test suite with
`--features net`, so both builds are checked on macOS arm64/x64, Windows x64,
and Linux x64.

## Privacy

Nothing in this module carries screen content. A policy names a channel, a mode
names a setting, and `NetworkError::as_str` returns a fixed class name. An
error never includes the address it was given, because an address can carry a
host name. The dial tests and the probe bind loopback only; they never leave
the machine.

## Limits

- The compiled gate is per crate feature, not per profile. A user who wants no
  network code at all builds without `net`; a profile cannot remove code, only
  refuse it.
- The switch is in-memory. A process restart resets it to engaged, which is the
  safe direction.
- The socket guard observes `connect(2)` only. A platform transport that
  resolves a name but never connects, or that uses a raw socket path the guard
  cannot see, is out of scope for this script; the build-time feature gate is
  the primary control.
- Nothing here authenticates or encrypts a channel. Transport security belongs
  to the platform TLS stack, and the optional sync channel is end-to-end
  encrypted (CC-089).
