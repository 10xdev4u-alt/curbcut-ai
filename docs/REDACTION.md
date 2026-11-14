# Redaction and no-content logging

The never list forbids content in logs, dumps, crash reports, and telemetry.
This document describes the layer that enforces it and the CI test that proves
the layer works.

## The rule

Diagnostics carry **counts and hashes only**. A log line may say that a frame
was 1920 by 1080 and that 42 elements were seen. It may not say what any of them
said, what the pixels were, or which app they came from beyond a stable app id.

Allowed in a diagnostic:

- Field names, capability names, rule ids, profile slugs, error summaries.
- Lengths, counts, dimensions, timestamps, and hashes.
- User-authored configuration tokens, so a typo can be found.

Never allowed:

- Accessibility labels, names, descriptions, or values.
- OCR text.
- Raw accessibility strings.
- Pixel bytes or image data.
- Identifiers tied to content.

## The layer

`crates/curbcut-core/src/redact.rs` provides three pieces.

| Item | Role |
| --- | --- |
| `Redactor` | Holds the strings that must never be written and replaces them. |
| `Logger` | Redacts a message before handing it to a sink. |
| `log_debug!` / `log_info!` / `log_warn!` / `log_error!` | Macros that route through the process-wide logger. |

```rust
let mut redactor = Redactor::new();
redactor.add_secret("alice@example.com");
let clean = redactor.redact("label=alice@example.com count=1");
assert_eq!(clean, "label=[redacted] count=1");
```

`Redactor::canaries()` loads the fixture described below. `RedactMode::Hash`
replaces a secret with a stable 16-character hash for values that are safe to
correlate; the default is the `[redacted]` placeholder.

Every diagnostic must go through `Logger` or `Redactor`. A bare `println!` or
`eprintln!` in a diagnostic path bypasses the layer and is a review blocker.

## The canary fixture

`crates/curbcut-core/tests/fixtures/content_canaries.txt` is the single source
of truth. One string per line, `#` for comments. The Rust loader and the CI
script read the same file.

Add a canary by adding a line. Do not remove one without replacing it, or a past
leak becomes undetectable.

## The CI test

`scripts/check_no_content_logging.py` scans files for the canaries.

```bash
python3 scripts/check_no_content_logging.py            # scan the repository
python3 scripts/check_no_content_logging.py PATH ...   # scan dumps or logs
python3 scripts/check_no_content_logging.py --self-test
```

`--self-test` writes a canary into a temporary file and requires the scanner to
find it, so a broken scanner cannot pass by silently matching nothing.

The `privacy` CI job runs the self-test, scans the repository, and runs
`cargo test -p curbcut-core --test no_content_logging`.

## Auditing real artifacts

Point the script at a directory of real dumps or logs before a release:

```bash
python3 scripts/check_no_content_logging.py /var/log/curbcut
```

A clean scan is a measurement, not a guarantee. Record the directory, the date,
and the canary count in the release notes.
