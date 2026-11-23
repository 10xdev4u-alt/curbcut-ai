# Headless CLI

The `curbcut-cli` binary is a thin wrapper over `curbcut_cli::run`. It exists so
the privacy and benchmark paths can be exercised without the desktop app.

## Arguments

Arguments are collected with `args_os` and converted lossily. A non-UTF-8
argument therefore reaches the parser and exits 2 as an unknown command, rather
than panicking in the argument iterator.

## Commands

| Command | Output | Exit |
| --- | --- | --- |
| `status` | `status ok`, `crate=…`, `version=…` | 0 |
| `dump` | redacted graph dump | 0 |
| `bench` | `bench iterations=… bytes=… elapsed_ns=…` | 0 |
| `vpat` | the draft VPAT for our own UI | 0 |
| `limitations` | the numbered limitations list | 0 |
| `panic` | a panic message | 101 |
| `help`, `--help`, `-h` | usage | 0 |

Bad arguments print `error: …` and the usage text, and exit 2. An unknown
command names itself. No argument path panics: parsing happens before dispatch
and every malformed input is a usage error.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | success |
| 2 | bad arguments |
| 101 | panic (the Rust runtime's code) |

The codes are stable and unit-tested. `Command::parse` returns `Err` for a
missing command, an unknown command, or extra arguments; `run_to` maps that to
`EXIT_USAGE`.

## The dump is redacted by construction

`DumpGraph::dump` prints lengths and hashes only:

```
graph name_len=12 name=[f21287eb1f5dd4c6]
nodes=3
node id=1 label_len=6 label=[3a506cb501761960] value_len=12 value=[5b9e6051ae9cd006]
```

It never prints a name, label, or value. The hash is the core redactor's keyless
`RedactMode::Hash`: deterministic, so the same value correlates across dumps,
but recoverable offline for low-entropy values. It is for correlation, not
protection. `the_dump_contains_neither_secret` seeds two known secrets and
asserts neither appears.

## The benchmark

`bench` runs the redaction workload `DEFAULT_ITERATIONS` (100,000) times.
`bytes` is the sum of the redacted lengths, which is deterministic for a fixed
iteration count, so two runs can be compared. `elapsed_ns` is wall-clock and
varies. `the_byte_count_is_deterministic` pins the checksum.

## The generated accessibility documents

`vpat` prints the draft VPAT and `limitations` prints the limitations list. Both
are rendered from `crates/curbcut-cli/src/vpat.rs`, the same data that
`scripts/render_a11y_mapping.py` writes to `docs/A11Y_MAPPING.md`,
`docs/VPAT.md`, and `docs/LIMITATIONS.md`. The command output and the published
documents cannot drift apart.

```bash
cargo run -q --bin curbcut-cli -- vpat
cargo run -q --bin curbcut-cli -- limitations
```

The VPAT is a draft. It evaluates CurbCut's own UI only, and it is never a
conformance claim about a third-party application. Manual rows point at a
procedure in `docs/A11Y_AUDIT.md`; see
[`A11Y_MAPPING.md`](A11Y_MAPPING.md).

## Testing

```bash
cargo test -p curbcut-cli
cargo clippy -p curbcut-cli --all-targets -- -D warnings
cargo run -q --bin curbcut-cli -- dump
```

The unit tests call `run_to` with `String` sinks, so they assert exit codes and
output without spawning a process. `the_panic_command_panics_with_a_content_free_message`
uses `catch_unwind` and asserts the payload is exactly the fixed message.
