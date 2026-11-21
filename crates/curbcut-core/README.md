# curbcut-core

Event bus, config, errors, capability tokens, orchestration.

## Status

Capability tokens, the typed command registry, versioned config, the
event bus, strongly typed ids, the redaction backstop, and the shared
value types are implemented. The network policy is implemented: the
`net` cargo feature is the compiled gate and a
`--no-default-features` build has no optional transport module at all,
`net::transport_available` reports which build is running,
`net::KillSwitch` is the runtime gate and starts engaged, and
`net::NetworkPolicy` defaults to offline. Only `net::update::dial`
exists without the feature, and it still consults the switch. See
[`../../docs/NETWORK_POLICY.md`](../../docs/NETWORK_POLICY.md).

## Build and test

```bash
cargo test -p curbcut-core
cargo clippy -p curbcut-core --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
