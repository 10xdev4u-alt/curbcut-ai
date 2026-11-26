# curbcut-cli

Headless control, benchmark, redacted debug dump, and the accessibility mapping/VPAT generator.

## Status

The headless CLI is implemented: status, redacted dump, benchmark,
process-local forget-layouts, forget-simplifications, and forget-inbox
deletion commands, the process-local network-status and
network-revoke inspect/revoke recipes, panic, vpat, and limitations
commands, with stable exit codes. The deletion commands act on
process-local RAM only: no app, no file, and F-10 stays OPEN. The
recipes parse a config document on stdin and never dial, write a
file, or change persisted state; F-12, R-09, and #221 stay open.
The accessibility mapping, the draft
VPAT, and the limitations page are generated from one data module, so a
published claim and the test that backs it cannot drift apart. See
[`../../docs/CLI.md`](../../docs/CLI.md),
[`../../docs/NETWORK_POLICY.md`](../../docs/NETWORK_POLICY.md),
[`../../docs/A11Y_MAPPING.md`](../../docs/A11Y_MAPPING.md), and
[`../../docs/VPAT.md`](../../docs/VPAT.md).

## Build and test

```bash
cargo test -p curbcut-cli
cargo clippy -p curbcut-cli --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
