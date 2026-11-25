# curbcut-cli

Headless control, benchmark, redacted debug dump, and the accessibility mapping/VPAT generator.

## Status

The headless CLI is implemented: status, redacted dump, benchmark,
process-local forget-layouts, panic, vpat, and limitations commands,
with stable exit codes. The accessibility mapping, the draft VPAT, and
the limitations page are generated from one data module, so a published
claim and the test that backs it cannot drift apart. See
[`../../docs/CLI.md`](../../docs/CLI.md),
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
