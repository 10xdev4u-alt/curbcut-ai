# curbcut-core

Event bus, config, errors, capability tokens, orchestration.

## Status

Capability tokens, the typed command registry, versioned config, the
event bus, strongly typed ids, the redaction backstop, and the shared
value types are implemented. The network policy is implemented and
opt-in: `net` is the compiled gate (`default = []`), so a plain build
has no optional transport module at all, and
`net::transport_available` reports which build is running.
`net::KillSwitch` is the runtime gate and starts engaged, and
`net::NetworkPolicy` defaults to offline. Only `net::update::dial`
exists without the feature, it takes the feature it dials and refuses
anything but `UpdateCheck`, and it still consults the switch. The
`socket-guard` CI job runs `scripts/socket_guard.py` over the
`net_probe` example in both builds. See
[`../../docs/NETWORK_POLICY.md`](../../docs/NETWORK_POLICY.md).

## Build and test

```bash
cargo test -p curbcut-core
cargo clippy -p curbcut-core --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
