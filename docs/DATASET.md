# Published dataset and benchmark

CurbCut publishes one fixture corpus and one deterministic benchmark. This page
is the researcher-facing entry point; the corpus itself ships a card at
[`crates/curbcut-core/tests/fixtures/README.md`](crates/curbcut-core/tests/fixtures/README.md)
and a machine-readable inventory at
[`crates/curbcut-core/tests/fixtures/manifest.json`](crates/curbcut-core/tests/fixtures/manifest.json).
`CC-116`.

## What is published

`curbcut-fixtures` v1.0.0, licensed **CC0-1.0-only**. It contains exactly one
file, `content_canaries.txt`: distinctive strings that must never appear in a
log, dump, crash report, or any other diagnostic. The redactor, the CI
no-content-logging test, and the golden plan snapshots all read it.

The corpus is **synthetic**. It holds no screen capture, no screenshot, no
accessibility string, and no personal data, so its `consent` field is
`not-required`. Nothing here was collected from a person.

## Reproducing the benchmark

```bash
python3 scripts/reproduce_benchmark.py
```

which builds and runs:

```bash
cargo run -q -p curbcut-cli -- bench
# bench iterations=100000 bytes=4100000 elapsed_ns=...
```

`bytes` is a checksum over the redacted output. It is deterministic for a fixed
iteration count and is asserted against the manifest on every run, so a change
in redaction behaviour fails loudly rather than quietly shifting the baseline.
`elapsed_ns` is wall-clock and varies by machine; the script reports it and does
not gate on it. To verify a recorded run without building:

```bash
python3 scripts/reproduce_benchmark.py --from build/bench.txt
```

## Protocol citation

Human-subject evaluation follows
[`docs/STUDY_PROTOCOL.md`](STUDY_PROTOCOL.md), which is pending ethics review.
The benchmark above is not that study and must not be cited as evidence about
users: it has no participants, no consent, and no outcome measure.

## Limits

- The corpus is small and synthetic. It exists to keep the privacy guarantees
  exercised, not to train or evaluate a model.
- There is no consented screen-capture corpus yet. Publishing one requires
  consent, a documented lawful basis, and a retention rule; none is claimed
  until those exist.
- The benchmark measures redaction throughput only. Capture latency, vision
  accuracy, and reading-level quality need separate suites with their own
  budgets.

## Verifying the publication

```bash
python3 scripts/validate_dataset.py
python3 scripts/validate_dataset.py --self-test
```

The validator checks the manifest hashes, the license and consent statements,
the required documentation sections, and that CI runs the check.
