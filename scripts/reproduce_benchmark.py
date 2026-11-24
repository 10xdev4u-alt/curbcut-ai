#!/usr/bin/env python3
"""Run the published benchmark and check it against the dataset manifest.

CC-116 publishes a deterministic benchmark alongside the fixture corpus. The
byte count is a sum of the redacted output lengths, so it is stable across
machines for a fixed iteration count and detects any change that alters output
length. This script runs the benchmark, compares the result with
``manifest.json``, and reports the environment that produced it.

Usage::

    python3 scripts/reproduce_benchmark.py
    python3 scripts/reproduce_benchmark.py --from build/bench.txt
    python3 scripts/reproduce_benchmark.py --self-test

``elapsed_ns`` is wall-clock and varies by machine. It is reported, never
compared to a budget; the acceptance gate is the deterministic byte count.

The benchmark is executed from a fixed argument vector, never through a shell:
the manifest records the command as documentation, and a manifest that claims
a different command fails instead of being trusted.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = (
    ROOT / "crates" / "curbcut-core" / "tests" / "fixtures" / "manifest.json"
)
BENCH_LINE = re.compile(
    r"bench iterations=(?P<iterations>\d+) bytes=(?P<bytes>\d+) "
    r"elapsed_ns=(?P<elapsed>\d+)"
)
BENCH_ARGV = ["cargo", "run", "-q", "-p", "curbcut-cli", "--", "bench"]


class BenchmarkError(Exception):
    """Raised when a run cannot be parsed or does not match the manifest."""


def parse_output(text: str) -> dict:
    match = BENCH_LINE.search(text)
    if match is None:
        raise BenchmarkError("no 'bench iterations=… bytes=… elapsed_ns=…' line found")
    return {
        "iterations": int(match.group("iterations")),
        "bytes": int(match.group("bytes")),
        "elapsed_ns": int(match.group("elapsed")),
    }


def load_manifest() -> dict:
    try:
        document = json.loads(MANIFEST.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise BenchmarkError(f"cannot read {MANIFEST.name}: {error}") from error
    if not isinstance(document, dict):
        raise BenchmarkError(f"{MANIFEST.name} is not a JSON object")
    return document


def compare(stats: dict, benchmark: dict) -> list[str]:
    errors: list[str] = []
    wanted_iterations = benchmark.get("iterations")
    wanted_bytes = benchmark.get("expected_bytes")
    if stats["iterations"] != wanted_iterations:
        errors.append(
            f"iterations {stats['iterations']} do not match the manifest "
            f"({wanted_iterations!r})"
        )
    if stats["bytes"] != wanted_bytes:
        errors.append(
            f"bytes {stats['bytes']} do not match the manifest ({wanted_bytes!r}); "
            "redaction behaviour changed"
        )
    return errors


def environment() -> str:
    try:
        rustc = subprocess.run(
            ["rustc", "--version"],
            capture_output=True,
            text=True,
            timeout=20,
            check=False,
        ).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        rustc = "unknown"
    cpus = os.cpu_count() or 0
    return (
        f"{platform.system()} {platform.release()} {platform.machine()} "
        f"cpus={cpus} rustc={rustc}"
    )


def command_matches(manifest_command: object) -> bool:
    """True only when the manifest records the fixed benchmark invocation."""
    return isinstance(manifest_command, str) and manifest_command == " ".join(BENCH_ARGV)


def run_benchmark(argv: list[str]) -> str:
    try:
        result = subprocess.run(
            argv,
            cwd=str(ROOT),
            capture_output=True,
            text=True,
            timeout=900,
            check=False,
        )
    except subprocess.SubprocessError as error:
        raise BenchmarkError(f"could not run the benchmark: {error}") from error
    if result.returncode != 0:
        raise BenchmarkError(
            f"benchmark exited {result.returncode}: {result.stderr.strip()[:200]}"
        )
    return result.stdout


def _self_test() -> int:
    checks = 0

    def expect(condition: bool, label: str) -> None:
        nonlocal checks
        if not condition:
            print(f"SELF-TEST FAILED: {label}", file=sys.stderr)
            raise SystemExit(1)
        checks += 1

    good = {"iterations": 100000, "expected_bytes": 4100000}
    expect(parse_output("bench iterations=100000 bytes=4100000 elapsed_ns=42\n") == {
        "iterations": 100000,
        "bytes": 4100000,
        "elapsed_ns": 42,
    }, "a well-formed line parses")
    expect(
        command_matches(" ".join(BENCH_ARGV)) is True,
        "the manifest's canonical command matches the argument vector",
    )
    expect(command_matches("echo hi") is False, "a substituted command is rejected")
    expect(compare({"iterations": 100000, "bytes": 4100000}, good) == [], "a matching run passes")
    expect(
        any("bytes" in error for error in compare({"iterations": 100000, "bytes": 1}, good)),
        "a rewritten byte count is reported",
    )
    expect(
        any("iterations" in error for error in compare({"iterations": 99, "bytes": 4100000}, good)),
        "a rewritten iteration count is reported",
    )
    for bad in ("", "status ok", "bench iterations=x bytes=1 elapsed_ns=1"):
        try:
            parse_output(bad)
        except BenchmarkError:
            checks += 1
        else:
            print(f"SELF-TEST FAILED: {bad!r} parsed", file=sys.stderr)
            raise SystemExit(1)

    print(f"self-test: {checks} benchmark checks passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--from",
        dest="source",
        type=Path,
        help="parse a recorded run instead of building and running the benchmark",
    )
    parser.add_argument("--self-test", action="store_true", help="run built-in checks")
    args = parser.parse_args()

    if args.self_test:
        return _self_test()

    try:
        benchmark = load_manifest().get("benchmark")
        if not isinstance(benchmark, dict):
            raise BenchmarkError(f"{MANIFEST.name} has no benchmark block")
        if not command_matches(benchmark.get("command")):
            raise BenchmarkError(
                "the manifest records a different benchmark command; refusing "
                "to run it"
            )
        output = (
            args.source.read_text(encoding="utf-8")
            if args.source
            else run_benchmark(BENCH_ARGV)
        )
        stats = parse_output(output)
        errors = compare(stats, benchmark)
    except (OSError, BenchmarkError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1

    print(
        f"bench iterations={stats['iterations']} bytes={stats['bytes']} "
        f"elapsed_ns={stats['elapsed_ns']}"
    )
    print(f"expected iterations={benchmark['iterations']} bytes={benchmark['expected_bytes']}")
    print(f"environment: {environment()}")
    if errors:
        for message in errors:
            print(f"FAIL: {message}", file=sys.stderr)
        return 1
    print("benchmark matches the published baseline")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
