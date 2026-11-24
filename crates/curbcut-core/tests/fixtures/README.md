# CurbCut fixture dataset

`curbcut-fixtures` v1.0.0 — the published fixture corpus of the CurbCut AI
repository, and the input to its deterministic benchmark.

Machine-readable inventory: [`manifest.json`](manifest.json).
Researcher-facing page: [`docs/DATASET.md`](../../../../docs/DATASET.md).

## License

The corpus is dedicated to the public domain under **CC0-1.0-only**
(<https://creativecommons.org/publicdomain/zero/1.0/>). You may copy, modify,
and redistribute it without asking. The rest of the repository remains under
`AGPL-3.0-only`; this dedication covers only the files this manifest lists.

## Consent

`consent` in the manifest is `not-required`, and that is a statement of fact,
not a waiver: every file here is synthetic and authored for this repository.
The corpus contains no screen capture, no screenshot, no accessibility-tree
string, no OCR output, and no personal data. Nothing was collected from a
participant, so nothing needed consent.

## Contents

| File | Bytes | SHA-256 (first 16) | Role |
| --- | --- | --- | --- |
| `content_canaries.txt` | 757 | `834008eab783c255` | Canary corpus: strings that must never appear in a diagnostic. |

Three consumers read `content_canaries.txt`:

- `Redactor::canaries()` in `crates/curbcut-core/src/redact.rs`
- `scripts/check_no_content_logging.py` (the CI no-content-logging test)
- `crates/curbcut-core/tests/golden_plans.rs` (the canary plan)

Add a canary by adding a line. Do not remove one without replacing it, or a
past leak becomes undetectable.

## Reproducing the benchmark

```bash
python3 scripts/reproduce_benchmark.py
```

The script builds and runs the CLI benchmark, checks the deterministic byte
count against `manifest.json`, and prints the environment alongside the
timings:

```
cargo run -q -p curbcut-cli -- bench
# bench iterations=100000 bytes=4100000 elapsed_ns=...
```

`bytes` is a sum of the redacted output lengths and is stable across machines
for a fixed iteration count; it detects any change that alters output length,
but not a change that preserves length. `elapsed_ns` is wall-clock and varies;
it is reported, never gated. To parse a previously recorded run instead of
building:

```bash
python3 scripts/reproduce_benchmark.py --from build/bench.txt
```

## Protocol citation

The evaluation protocol for human studies is
[`docs/STUDY_PROTOCOL.md`](../../../../docs/STUDY_PROTOCOL.md). It is cited here
because the benchmark above is *not* that study: it is a deterministic
software measurement with no participants. Do not cite the benchmark as
evidence about users.

## Limits

- This corpus is small and synthetic. It exists so the privacy and redaction
  guarantees are continuously exercised, not so a model can be trained.
- There is **no consented screen-capture corpus** in this repository yet. A
  real corpus would need consent, a lawful basis, and a retention rule before
  it is published; until then, no such dataset is claimed.
- The benchmark measures one workload (redaction throughput). It says nothing
  about capture latency, vision accuracy, or reading-level quality; those need
  their own suites.
