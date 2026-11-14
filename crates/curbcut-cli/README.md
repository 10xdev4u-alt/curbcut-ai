# curbcut-cli

Headless control, benchmark, and redacted debug dump.

## Status

The headless CLI is implemented: status, redacted dump, benchmark, and
panic commands, with stable exit codes. See
[`../../docs/CLI.md`](../../docs/CLI.md).

## Build and test

```bash
cargo test -p curbcut-cli
cargo clippy -p curbcut-cli --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
