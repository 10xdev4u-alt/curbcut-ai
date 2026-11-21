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
  transport in the binary at all. A `--no-default-features` build has no
  `transport` module, so an optional feature has no code path that can reach a
  socket.
- **The runtime gate.** `KillSwitch` holds a `NetworkPolicy` and an engaged
  flag. It starts engaged, so a fresh switch denies everything even with an
  online policy; `release()` hands the decision to the policy, and `engage()`
  denies everything again.

## The channels

| Feature | String | Offline build | Purpose |
| --- | --- | --- | --- |
| `ModelDownload` | `model_download` | removed | fetch a model, vocabulary, or voice |
| `UpdateCheck` | `update_check` | kept | check for a newer build |
| `Sync` | `sync` | removed | optional encrypted profile sync |
| `Telemetry` | `telemetry` | removed | opt-in, counts-only telemetry |
| `LicenseCheck` | `license_check` | removed | validate a licence or donation key |

`UpdateCheck` is the one channel every build keeps: it is how a broken install
is repaired. It still consults the switch, so an engaged switch blocks updates
too.

## The modes

| Mode | String | Allows |
| --- | --- | --- |
| `Offline` | `offline` | nothing |
| `UpdatesOnly` | `updates_only` | `UpdateCheck` |
| `Online` | `online` | every channel, subject to the switch |

`NetworkPolicy` is data. It holds no socket, no clock, and no I/O, so the same
policy always answers the same question and can be stored in the profile graph.

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

`transport::dial` and `update::dial` consult the switch at the boundary, so a
caller cannot forget the gate. Both return `NetworkError::Blocked` without
dialing when the switch refuses.

## What the offline build removes

`cargo build --no-default-features` produces a binary where `net::transport`
does not exist. The only remaining socket path is `net::update::dial`, and it
is still behind the switch. `net::transport_available()` reports which build is
running, and the test suite asserts both directions:

- with the `net` feature, the optional transport exists and follows the switch;
- without it, no optional transport exists and only the update channel can
  dial.

## Privacy

Nothing in this module carries screen content. A policy names a channel, a mode
names a setting, and `NetworkError::as_str` returns a fixed class name. An
error never includes the address it was given, because an address can carry a
host name. The dial tests bind loopback only; they never leave the machine.

## Limits

- The compiled gate is per crate feature, not per profile. A user who wants no
  network code at all builds with `--no-default-features`; a profile cannot
  remove code, only refuse it.
- The switch is in-memory. A process restart resets it to engaged, which is the
  safe direction.
- Nothing here authenticates or encrypts a channel. Transport security belongs
  to the platform TLS stack, and the optional sync channel is end-to-end
  encrypted (CC-089).
