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

The scan is deliberately broader than runtime redaction: matching is
case-insensitive, because a log that lowercases a canary still leaked it. A file
that cannot be read is reported and fails the run unless ``--allow-skip`` is
given; a silent skip would be a false all-clear.
"""

from __future__ import annotations

import argparse
import os
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
CHUNK_BYTES = 1 << 20
# A canary can straddle a chunk boundary; carry this many bytes across.
OVERLAP_BYTES = 256


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


def is_fixture(path: Path) -> bool:
    """Whether `path` is the fixture or a hard link to it."""
    try:
        candidate = path.stat()
        fixture = CANARY_FILE.stat()
    except OSError:
        return False
    return (candidate.st_dev, candidate.st_ino) == (fixture.st_dev, fixture.st_ino)


def iter_files(paths: list[Path]):
    for path in paths:
        if path.is_file():
            yield path
        elif path.is_dir():
            yield from walk(path)
        else:
            raise SystemExit(f"no such path: {path}")


def walk(root: Path):
    """Yield files under `root`, following symlinked directories once."""
    stack = [root]
    seen: set[tuple[int, int]] = set()
    while stack:
        directory = stack.pop()
        try:
            entries = sorted(directory.iterdir())
        except OSError:
            continue
        for entry in entries:
            try:
                if entry.is_dir():
                    if entry.name in SKIP_DIRS:
                        continue
                    if entry.is_symlink():
                        stat = entry.stat()
                        key = (stat.st_dev, stat.st_ino)
                        if key in seen:
                            continue
                        seen.add(key)
                    stack.append(entry)
                elif entry.is_file():
                    yield entry
            except OSError:
                continue


def scan(
    paths: list[Path], canaries: list[str]
) -> tuple[list[tuple[Path, str]], list[tuple[Path, str]]]:
    """Return `(leaks, skipped)`.

    Files are read in chunks, so an oversized log is scanned rather than
    skipped. Matching is case-insensitive.
    """
    leaks: list[tuple[Path, str]] = []
    skipped: list[tuple[Path, str]] = []
    needles = [(canary, canary.lower()) for canary in canaries]
    for path in iter_files(paths):
        if is_fixture(path):
            continue
        try:
            with path.open("rb") as handle:
                carry = b""
                found: set[str] = set()
                while True:
                    chunk = handle.read(CHUNK_BYTES)
                    if not chunk:
                        break
                    text = (carry + chunk).decode("utf-8", errors="replace").lower()
                    for canary, needle in needles:
                        if needle in text:
                            found.add(canary)
                    carry = chunk[-OVERLAP_BYTES:]
        except OSError as error:
            skipped.append((path, str(error)))
            continue
        for canary in canaries:
            if canary in found:
                leaks.append((path, canary))
    return leaks, skipped


def self_test(canaries: list[str]) -> int:
    """Seed leaks and require the scanner to find them."""
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        nested = root / "nested"
        nested.mkdir()
        (nested / "crash.log").write_text(
            f"unrelated line\n{canaries[0]}\n", encoding="utf-8"
        )
        (root / "clean.log").write_text("nothing here\n", encoding="utf-8")
        (root / "variant.log").write_text(
            f"{canaries[0].lower()}\n", encoding="utf-8"
        )
        # A hard link to the fixture must be excluded by inode, not by name.
        os.link(CANARY_FILE, root / "fixture-link.txt")

        leaks, skipped = scan([root], canaries)
        found = {canary for _, canary in leaks}
        if canaries[0] not in found:
            print(
                "SELF-TEST FAILED: a seeded leak was not detected",
                file=sys.stderr,
            )
            return 1
        if skipped:
            print(f"SELF-TEST FAILED: files were skipped: {skipped}", file=sys.stderr)
            return 1
        print(
            f"self-test: detected {canaries[0]!r} in {len(leaks)} report(s), "
            "fixture link excluded"
        )
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
    parser.add_argument(
        "--allow-skip",
        action="store_true",
        help="exit 0 when a file could not be read",
    )
    args = parser.parse_args()

    canaries = load_canaries(CANARY_FILE)
    if args.self_test:
        return self_test(canaries)

    targets = args.paths or [ROOT]
    leaks, skipped = scan(targets, canaries)
    for path, canary in leaks:
        print(f"LEAK: {canary!r} found in {path}", file=sys.stderr)
    for path, reason in skipped:
        print(f"SKIP: {path}: {reason}", file=sys.stderr)
    if leaks:
        print(
            f"{len(leaks)} content leak(s); diagnostics carry counts and hashes only",
            file=sys.stderr,
        )
        return 1
    if skipped and not args.allow_skip:
        print(
            f"{len(skipped)} file(s) could not be read; a silent skip is not an "
            "all-clear (use --allow-skip to acknowledge)",
            file=sys.stderr,
        )
        return 1
    scanned = sum(1 for _ in iter_files(targets))
    print(
        f"clean: {len(canaries)} canaries absent from {scanned} file(s) "
        f"under {len(targets)} path(s)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
