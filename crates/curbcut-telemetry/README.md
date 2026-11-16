# curbcut-telemetry

Opt-in, counts-only telemetry and redaction.

## Status

The counts-only telemetry schema is implemented: `MetricKey` is a
16-entry allowlist, `Snapshot` serializes keys and numbers and parses
the same format back while rejecting an unknown key, and
`TelemetryExporter` renders an export only after an opt-in and
clears the counts on opt-out. See
[`../../docs/TELEMETRY.md`](../../docs/TELEMETRY.md).

## Build and test

```bash
cargo test -p curbcut-telemetry
cargo clippy -p curbcut-telemetry --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
