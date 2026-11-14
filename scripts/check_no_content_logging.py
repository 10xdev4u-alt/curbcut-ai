#!/usr/bin/env python3
"""Fail if a content canary appears in a diagnostic artifact.

The canary list is the single source of truth shared with the Rust redaction
layer: ``crates/curbcut-core/tests/fixtures/content_canaries.txt``.

Usage::

    python3 scripts/check_no_content_logging.py              # scan the repo
    python3 scripts/check_no_content_logging.py PATH [...]   # scan paths
    python3 scripts/check_no_content_logging.py --self-test  # prove it fails

The default scan covers the whole repository except the canary fixture itself
and VCS/build directories, so a canary committed by accident fails CI. Point it
at a directory of dumps, logs, or crash reports to audit real artifacts.
"""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CANARY_FILE = (
    ROOT / "crates" / "curbcut-core" / "tests" / "fixtures" / "content_canaries.txt"
)

SKIP_DIRS = {
    ".git",
    "target",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    ".mypy_cache",
    ".pytest_cache",
}
MAX_BYTES = 8 * 1024 * 1024


def load_canaries(path: Path) -> list[str]:
    if not path.exists():
        raise SystemExit(f"canary fixture not found: {path}")
    canaries = [
        line.strip()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.strip().startswith("#")
    ]
    if not canaries:
        raise SystemExit(f"no canaries in {path}")
    return canaries


def iter_files(paths: list[Path]):
    for path in paths:
        if path.is_file():
            yield path
        elif path.is_dir():
            for child in sorted(path.rglob("*")):
                if not child.is_file():
                    continue
                if any(part in SKIP_DIRS for part in child.parts):
                    continue
                yield child
        else:
            raise SystemExit(f"no such path: {path}")


def scan(paths: list[Path], canaries: list[str]) -> list[tuple[Path, str]]:
    leaks: list[tuple[Path, str]] = []
    for path in iter_files(paths):
        if path.resolve() == CANARY_FILE.resolve():
            continue
        try:
            data = path.read_bytes()
        except OSError:
            continue
        if b"\x00" in data[:1024] or len(data) > MAX_BYTES:
            continue
        text = data.decode("utf-8", errors="replace")
        for canary in canaries:
            if canary in text:
                leaks.append((path, canary))
    return leaks


def self_test(canaries: list[str]) -> int:
    """Seed a leak and require the scanner to find it."""
    with tempfile.TemporaryDirectory() as tmp:
        artifact = Path(tmp) / "crash.log"
        artifact.write_text(
            f"unrelated line\n{canaries[0]}\n", encoding="utf-8"
        )
        leaks = scan([artifact], canaries)
        if not leaks:
            print("SELF-TEST FAILED: a seeded leak was not detected", file=sys.stderr)
            return 1
        print(f"self-test: detected {leaks[0][1]!r} in {leaks[0][0].name}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "paths",
        nargs="*",
        type=Path,
        help="files or directories to scan (default: the repository)",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="seed a leak and require detection",
    )
    args = parser.parse_args()

    canaries = load_canaries(CANARY_FILE)
    if args.self_test:
        return self_test(canaries)

    targets = args.paths or [ROOT]
    leaks = scan(targets, canaries)
    for path, canary in leaks:
        print(f"LEAK: {canary!r} found in {path}", file=sys.stderr)
    if leaks:
        print(
            f"{len(leaks)} content leak(s); diagnostics carry counts and hashes only",
            file=sys.stderr,
        )
        return 1
    print(f"clean: {len(canaries)} canaries absent from {len(targets)} path(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
