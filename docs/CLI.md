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
| `forget-layouts` | `deleted layouts=N`, `remaining layouts=0`, scope and reach lines | 0 |
| `forget-layouts-app` | `deleted layouts=N`, `remaining layouts=M`, scope and reach lines | 0 |
| `forget-simplifications` | `deleted feedback entries=N`, `remaining feedback entries=0`, scope and reach lines | 0 |
| `forget-inbox` | `deleted inbox entries=N`, `remaining inbox entries=0`, scope and reach lines | 0 |
| `forget-inbox-entry` | `removed inbox entries=N`, `remaining inbox entries=M`, scope and reach lines | 0 |
| `network-status` | `update_check=allowed\|blocked`, `policy=…`, `switch=…`, `scope=process-local RAM only` | 0 |
| `network-revoke` | `update_check = false`, `scope=process-local RAM only` | 0 |
| `vpat` | the draft VPAT for our own UI | 0 |
| `limitations` | the numbered limitations list | 0 |
| `panic` | a panic message | 101 |
| `help`, `--help`, `-h` | usage | 0 |

Bad arguments print `error: …` and the usage text, and exit 2. An unknown
command names itself. No argument path panics: parsing happens before dispatch
and every malformed input is a usage error.

## Process-local deletion

`forget-layouts` reads `<user:u64> <app-slug> <window:u64>` lines from stdin,
builds an in-memory `LayoutMemory`, and calls `LayoutMemory::reset_all`:

```bash
printf '1 mail 7\n2 mail 8\n' | cargo run -q --bin curbcut-cli -- forget-layouts
# deleted layouts=2
# remaining layouts=0
# scope=process-local RAM only
# reach=no running app or store file
```

Blank lines are skipped; duplicate keys collapse, so `N` counts distinct
layouts, not input lines. At most 4096 non-blank lines are accepted; this is a
semantic cap, not a memory guard against an arbitrarily large stdin. A malformed line
reports its line number only and exits 2; identifiers are never echoed.

This is a **process-local, RAM-only** affordance for DC-14. It cannot reach a
running app's memory, reads and writes no file, and does not delete any other
class. It is not an in-app deletion control, sets no maximum lifetime, and does
not close F-10. See [`DATA_INVENTORY.md`](DATA_INVENTORY.md) §Until-deleted
classes.

`forget-layouts-app` reads the same entry lines as `forget-layouts`, followed
by a final `<user:u64> <app-slug>` line selecting one user's layouts for one
app. It builds one `LayoutMemory` from the entry lines and calls
`LayoutMemory::reset_app(&UserId(user), &AppId::new(slug))`:

```bash
printf '1 mail 7\n1 mail 8\n2 chat 9\n1 mail\n' | cargo run -q --bin curbcut-cli -- forget-layouts-app
# deleted layouts=2
# remaining layouts=1
# scope=process-local RAM only
# reach=no running app or store file
```

Duplicate keys collapse before the reset, so `N` counts the distinct layouts
dropped for that one user and app and `M` counts the layouts kept for other
users and apps. The selector line counts toward the 4096-line cap. Empty stdin
exits 2 with `error: no layout selector`; a malformed selector reports its line
number only as `error: malformed layout selector line N`, echoing no user or
slug. A selector that matches nothing succeeds with `deleted layouts=0`.
Like `forget-layouts`, this is **process-local, RAM-only**: it reaches no
running app and reads or writes no store file.

`forget-simplifications` reads `<hash:u64> <accept|reject>` lines from stdin,
builds a `LearnedSimplification` with capacity 4096, counts its entries with
`len`, and calls `clear` (which also drops undo history and resets the level):

```bash
printf '42 accept\n99 reject\n' | cargo run -q --bin curbcut-cli -- forget-simplifications
# deleted feedback entries=2
# remaining feedback entries=0
# scope=process-local RAM only
# reach=no running app or store file
```

The log does not coalesce entries, so duplicate lines count separately. The
capacity equals the line cap, so no accepted line is evicted before the count.

`forget-inbox` reads
`<app-slug> <kind> <x:i32> <y:i32> <w:u32> <h:u32> <dismissible:0|1>` lines
from stdin. `kind` is one fixed token: `toast`, `banner`, `popup`, `cookie`, or
`paywall`. Each line calls `Inbox::file`, then the count is taken with `len`
and `Inbox::clear` removes every entry:

```bash
printf 'mail toast 1 2 3 4 1\nchat banner -3 4 5 6 0\n' | cargo run -q --bin curbcut-cli -- forget-inbox
# deleted inbox entries=2
# remaining inbox entries=0
# scope=process-local RAM only
# reach=no running app or store file
```

`forget-inbox-entry` reads the same entry lines as `forget-inbox`, followed by
a final line `<id:u64>` selecting one of the filed entries. It files those
entries with `Inbox::file`, calls `Inbox::remove(InboxId(id))`, and reports the
removal count plus the number left after that removal:

```bash
printf 'mail toast 1 2 3 4 1\nchat banner -3 4 5 6 0\n0\n' | cargo run -q --bin curbcut-cli -- forget-inbox-entry
# removed inbox entries=1
# remaining inbox entries=1
# scope=process-local RAM only
# reach=no running app or store file
```

The id line counts toward the 4096-line cap. Empty stdin exits 2 with
`error: no entry id`; a malformed id or entry reports its line number only and
exits 2, echoing no id, slug, kind, or coordinate. An id that is not filed
succeeds with `removed inbox entries=0`. This command is **process-local,
RAM-only** and cannot reach a running app or store file: it builds a fresh
inbox from stdin, removes one entry from that inbox, and writes nothing.

The inbox files every line, so duplicate lines count separately. All five
deletion commands skip blank lines, accept CRLF, and stop at 4096 non-blank
lines. A malformed line reports its line number only and exits 2; no hash,
slug, kind, window, or coordinate is echoed. For the three clear-all commands,
empty stdin is a successful zero deletion; the two remove-one commands
(`forget-inbox-entry` and `forget-layouts-app`) need a final selector. Invalid
UTF-8 stdin exits 2.

These commands are process-local and RAM-only affordances for DC-14, DC-15,
and DC-24. They **cannot reach a running app or a store file**: there is no
app IPC, no database path, and no file write. They are not in-app deletion
controls, set no maximum lifetime, and do not close F-10.

## Network inspect and revoke recipes

`network-status` and `network-revoke` read a TOML config document on stdin.
`network-status` calls `Config::from_toml` and `KillSwitch::from_config`, then
prints only fixed labels:

```bash
printf 'version = 1\nupdate_check = true\n' | cargo run -q --bin curbcut-cli -- network-status
# update_check=allowed
# policy=updates-only
# switch=released
# scope=process-local RAM only
```

An absent `update_check` key or an explicit `false` prints
`update_check=blocked`, `policy=offline`, and `switch=engaged`.
`network-revoke` parses the same input and prints the canonical line
`update_check = false` with the same scope note:

```bash
printf 'version = 1\nupdate_check = true\n' | cargo run -q --bin curbcut-cli -- network-revoke
# update_check = false
# scope=process-local RAM only
```

Neither command dials, writes a file, or echoes a config token or value;
malformed input exits 2 with a line or field name only. This is a process-local
inspect/revoke **recipe**, not a shipped user control: it reaches no running
app and changes no persisted state. F-12, R-09, and #221 stay as recorded in
[`NETWORK_POLICY.md`](NETWORK_POLICY.md).

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
graph name_len=12 name_hash=[f21287eb1f5dd4c6]
nodes=3
node id=1 label_len=6 label_hash=[3a506cb501761960] value_len=12 value_hash=[5b9e6051ae9cd006]
```

It never prints a name, label, or value. The hash is the core redactor's keyless
`RedactMode::Hash`: deterministic, so the same value correlates across dumps,
but recoverable offline for low-entropy values. It is for correlation, not
protection; see [`docs/HASHES.md`](HASHES.md). `the_dump_contains_neither_secret`
seeds two known secrets and asserts neither appears.

## The benchmark

`bench` runs the redaction workload `DEFAULT_ITERATIONS` (100,000) times.
`bytes` is the sum of the redacted lengths, which is deterministic for a fixed
iteration count, so two runs can be compared. `elapsed_ns` is wall-clock and
varies. `the_byte_count_is_deterministic` pins the byte count.

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
