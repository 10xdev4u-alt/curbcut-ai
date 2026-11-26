# curbcut-core

Event bus, config, errors, capability tokens, orchestration, zeroizing buffers, golden plan snapshots, SHA-256/HMAC, the seeded property-test harness, and failure injection.

## Status

Capability tokens, the typed command registry, versioned config, the
event bus, strongly typed ids, the redaction backstop, and the shared
value types are implemented. The network policy is implemented and
opt-in: `net` is the compiled gate (`default = []`), so a plain build
has no optional transport module at all, and
`net::transport_available` reports which build is running.
`net::KillSwitch` is the runtime gate and starts engaged, and
`net::NetworkPolicy` defaults to offline. `KillSwitch::from_config`
can release it only when the root `update_check` preference is true;
no shipped binary calls that path yet. Only `net::update::dial`
exists without the feature, it takes the feature it dials and refuses
anything but `UpdateCheck`, and it still consults the switch. The
`socket-guard` CI job runs `scripts/socket_guard.py` over the
`net_probe` example in both builds. The `secure` module adds the
zeroizing `SecureBuffer` and `SecureText` types with a shared
`GUARD_BYTE` and a thread-local drop counter. The `snapshot` module
renders a content-free, deterministic `OverlayPlan` for the golden
tests. The `hash` module is a dependency-free SHA-256 and HMAC-SHA-256,
checked against the published test vectors. The `prop` module is the
seeded property-test harness every crate uses: a splitmix64 generator,
a runner that reports the seed and case index on a break, and a
wall-clock budget for fuzz runs. The `chaos` module is the failure
injection harness: scripted faults, a panic guard that drops the
payload, and a report that fails when a fault is unannounced or an
announcement does not recover. The `crash_report` module is a local,
content-free decision primitive: `CrashReportGate::prepare` yields a
report only after an in-process approval and consumes that one-shot
approval. There is no uploader, socket, recipient, approval UI, or
channel, and #219 stays open. See
[`../../docs/CHAOS_TESTS.md`](../../docs/CHAOS_TESTS.md),
[`../../docs/PROPERTY_TESTS.md`](../../docs/PROPERTY_TESTS.md),
[`../../docs/NETWORK_POLICY.md`](../../docs/NETWORK_POLICY.md),
[`../../docs/SECURE_BUFFERS.md`](../../docs/SECURE_BUFFERS.md),
[`../../docs/GOLDEN_TESTS.md`](../../docs/GOLDEN_TESTS.md), and
[`../../docs/MODEL_GUARD.md`](../../docs/MODEL_GUARD.md).

## Build and test

```bash
cargo test -p curbcut-core
cargo clippy -p curbcut-core --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
