#!/usr/bin/env python3
"""Fail when a guarded panic prints its payload.

`chaos::guard` must withhold the panic message: a payload can quote screen
content, and the default hook writes it to stderr before `catch_unwind` runs.
The Rust tests can only assert that the panic was caught, not that nothing was
printed, so this wrapper runs the probe as a subprocess and reads its stderr.

Usage:

    python3 scripts/check_panic_silence.py [PROBE]
    python3 scripts/check_panic_silence.py --self-test

`PROBE` defaults to the debug build of
`crates/curbcut-core/examples/panic_payload_probe.rs`. Build it first:

    cargo build -p curbcut-core --example panic_payload_probe

Exit codes:
    0  the probe ran and printed no canary, or the self-test passed
    1  a canary reached stdout/stderr, or the self-test failed
    2  the probe could not be run
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CANARY_FILE = (
    ROOT / "crates" / "curbcut-core" / "tests" / "fixtures" / "content_canaries.txt"
)
DEFAULT_PROBE = (
    ROOT / "target" / "debug" / "examples" / "panic_payload_probe"
)


class GuardError(RuntimeError):
    """The guard could not run."""


def load_canaries() -> list[str]:
    if not CANARY_FILE.exists():
        raise GuardError(f"canary fixture not found: {CANARY_FILE}")
    return [
        line.strip()
        for line in CANARY_FILE.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.strip().startswith("#")
    ]


def leaks(text: str, canaries: list[str]) -> list[str]:
    lowered = text.lower()
    return [canary for canary in canaries if canary.lower() in lowered]


def run_probe(probe: Path, canaries: list[str]) -> list[str]:
    if not probe.exists():
        raise GuardError(
            f"probe not built: {probe} "
            "(run: cargo build -p curbcut-core --example panic_payload_probe)"
        )
    try:
        completed = subprocess.run(
            [str(probe)],
            capture_output=True,
            text=True,
            timeout=60,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise GuardError(f"could not run {probe}: {error}") from error
    # A probe that aborts is itself evidence the panic escaped containment.
    if completed.returncode != 0:
        print(
            f"LEAK: probe exited {completed.returncode}; the guarded panic may have "
            "escaped",
            file=sys.stderr,
        )
        return ["<non-zero exit>"]
    found = leaks(completed.stdout + completed.stderr, canaries)
    for canary in found:
        print(f"LEAK: {canary!r} reached the probe's output", file=sys.stderr)
    return found


def self_test(canaries: list[str]) -> int:
    if not canaries:
        print("SELF-TEST FAILED: the fixture has no canaries", file=sys.stderr)
        return 1

    handle = tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False)
    handle.write(canaries[0])
    handle.close()
    try:
        loud = subprocess.run(
            [sys.executable, "-c", f"print(open({handle.name!r}).read())"],
            capture_output=True,
            text=True,
            check=True,
        )
        quiet = subprocess.run(
            [sys.executable, "-c", "print('nothing sensitive here')"],
            capture_output=True,
            text=True,
            check=True,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise GuardError(f"self-test could not run a child process: {error}") from error
    finally:
        Path(handle.name).unlink(missing_ok=True)

    if not leaks(loud.stdout + loud.stderr, canaries):
        print("SELF-TEST FAILED: a printed canary was not detected", file=sys.stderr)
        return 1
    print("self-test: a printed canary was detected")

    if leaks(quiet.stdout + quiet.stderr, canaries):
        print("SELF-TEST FAILED: a clean child was reported as leaking", file=sys.stderr)
        return 1
    print("self-test: a clean child produced no match")

    # The real probe must exist for the committed configuration to pass CI, so
    # the self-test fails loudly when it has not been built yet.
    if not DEFAULT_PROBE.exists():
        print(
            f"SELF-TEST FAILED: probe not built at {DEFAULT_PROBE}",
            file=sys.stderr,
        )
        return 1
    found = run_probe(DEFAULT_PROBE, canaries)
    if found:
        print("SELF-TEST FAILED: the committed probe leaked", file=sys.stderr)
        return 1
    print(f"self-test: {DEFAULT_PROBE} printed no canary")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "probe",
        nargs="?",
        type=Path,
        default=DEFAULT_PROBE,
        help=f"path to the probe binary (default: {DEFAULT_PROBE})",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="prove the detector catches a printed canary",
    )
    args = parser.parse_args()

    try:
        canaries = load_canaries()
        if args.self_test:
            return self_test(canaries)
        found = run_probe(args.probe, canaries)
    except GuardError as error:
        print(f"GUARD ERROR: {error}", file=sys.stderr)
        return 2

    if found:
        print(
            f"{len(found)} canary leak(s); a guarded panic reached the log",
            file=sys.stderr,
        )
        return 1
    print(f"no leaks: {len(canaries)} canaries absent from the probe's output")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
