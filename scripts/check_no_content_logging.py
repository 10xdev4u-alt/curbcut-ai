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
or directory that cannot be read is reported and fails the run unless
``--allow-skip`` is given; a silent skip would be a false all-clear.

The fixture is excluded by device and inode. On a filesystem where ``st_ino`` is
always 0 the exclusion falls back to the resolved path, which cannot identify a
hard link to the fixture; a hard link would then be reported as a leak.
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
    # `st_ino` is 0 on some filesystems; fall back to the resolved path. That
    # fallback cannot identify a hard link to the fixture, so a hard link is
    # reported there rather than silently excluded.
    if candidate.st_ino == 0 or fixture.st_ino == 0:
        return path.resolve() == CANARY_FILE.resolve()
    return (candidate.st_dev, candidate.st_ino) == (fixture.st_dev, fixture.st_ino)


def dir_key(path: Path, stat: os.stat_result) -> tuple[int, int] | str:
    """A dedupe key for a directory.

    `st_ino` is 0 on some filesystems. There the resolved path is the only
    identity available; using the raw pair would collapse every directory to
    one key and silently skip the scan.
    """
    if stat.st_ino == 0:
        return str(path.resolve())
    return (stat.st_dev, stat.st_ino)


def walk(root: Path, skipped: list[tuple[Path, str]]):
    """Yield files under `root`, following symlinked directories once.

    Directories reachable by more than one path are visited once, so a symlink
    alias cannot double-count the same file.
    """
    stack = [root]
    seen: set[tuple[int, int] | str] = set()
    while stack:
        directory = stack.pop()
        try:
            stat = directory.stat()
        except OSError as error:
            skipped.append((directory, str(error)))
            continue
        key = dir_key(directory, stat)
        if key in seen:
            continue
        seen.add(key)
        try:
            entries = sorted(directory.iterdir())
        except OSError as error:
            skipped.append((directory, str(error)))
            continue
        for entry in entries:
            try:
                if entry.is_dir():
                    if entry.name in SKIP_DIRS:
                        continue
                    stack.append(entry)
                elif entry.is_file():
                    yield entry
                else:
                    skipped.append((entry, "not a regular file or directory"))
            except OSError as error:
                skipped.append((entry, str(error)))


def scan_file(
    path: Path,
    needles: list[tuple[str, str]],
    overlap: int,
    leaks: list[tuple[Path, str]],
    skipped: list[tuple[Path, str]],
) -> None:
    """Scan one file in chunks and append leaks and skips."""
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
                carry = chunk[-overlap:]
    except OSError as error:
        skipped.append((path, str(error)))
        return
    for canary, _ in needles:
        if canary in found:
            leaks.append((path, canary))


def scan(
    paths: list[Path], canaries: list[str]
) -> tuple[list[tuple[Path, str]], list[tuple[Path, str]]]:
    """Return `(leaks, skipped)`."""
    leaks: list[tuple[Path, str]] = []
    skipped: list[tuple[Path, str]] = []
    needles = [(canary, canary.lower()) for canary in canaries]
    # Carry enough bytes across a chunk boundary to cover the longest canary.
    overlap = max([len(canary) for canary in canaries] + [1])

    for path in paths:
        if path.is_file():
            if not is_fixture(path):
                scan_file(path, needles, overlap, leaks, skipped)
        elif path.is_dir():
            for child in walk(path, skipped):
                if not is_fixture(child):
                    scan_file(child, needles, overlap, leaks, skipped)
        else:
            raise SystemExit(f"no such path: {path}")
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

        link_note = None
        try:
            os.link(CANARY_FILE, root / "fixture-link.txt")
        except OSError as error:
            link_note = (
                f"hard links unsupported ({error}); "
                "fixture-link exclusion was not checked"
            )

        leaks, skipped = scan([root], canaries)
        if skipped:
            print(f"SELF-TEST FAILED: files were skipped: {skipped}", file=sys.stderr)
            return 1
        leaked = {path.name for path, _ in leaks}
        if "crash.log" not in leaked:
            print("SELF-TEST FAILED: a seeded leak was not detected", file=sys.stderr)
            return 1
        if "variant.log" not in leaked:
            print(
                "SELF-TEST FAILED: a lowercased canary was not detected",
                file=sys.stderr,
            )
            return 1
        if "fixture-link.txt" in leaked:
            print(
                "SELF-TEST FAILED: a hard link to the fixture was reported",
                file=sys.stderr,
            )
            return 1
        if len(leaks) != 2:
            print(
                f"SELF-TEST FAILED: expected 2 leaks, found {len(leaks)}",
                file=sys.stderr,
            )
            return 1

        # A canary straddling a chunk boundary must still be found. Use the
        # longest canary, so the carry must cover it rather than the shortest.
        # Place it so all but its last byte fall in the first chunk.
        longest = max(canaries, key=len)
        boundary = root / "boundary.log"
        split = len(longest) - 1
        with boundary.open("wb") as handle:
            handle.write(b"x" * (CHUNK_BYTES - split))
            handle.write(longest.encode("utf-8"))
        leaks, skipped = scan([boundary], canaries)
        if skipped or not any(canary == longest for _, canary in leaks):
            print(
                "SELF-TEST FAILED: a canary straddling a chunk boundary was missed",
                file=sys.stderr,
            )
            return 1

        # An unreadable file must be reported as SKIP, not silently ignored.
        # Root can read a chmod-000 file, so the check is skipped there.
        if hasattr(os, "geteuid") and os.geteuid() == 0:
            print(
                "self-test: running as root; unreadable-file reporting not checked"
            )
        else:
            locked = root / "locked.log"
            locked.write_text("nothing\n", encoding="utf-8")
            locked.chmod(0)
            try:
                _leaks, skipped = scan([locked], canaries)
            finally:
                locked.chmod(0o644)
            if not skipped:
                print(
                    "SELF-TEST FAILED: an unreadable file was not reported",
                    file=sys.stderr,
                )
                return 1
            print("self-test: an unreadable file was reported as SKIP")

        print(
            f"self-test: detected {canaries[0]!r} in crash.log and variant.log, "
            f"and a boundary-straddling {longest!r}"
        )
        if link_note:
            print(f"self-test: {link_note}")
        else:
            print("self-test: the fixture hard link was excluded")
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
        help="exit 0 when a file or directory could not be read",
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
            f"{len(skipped)} path(s) could not be read; a silent skip is not an "
            "all-clear (use --allow-skip to acknowledge)",
            file=sys.stderr,
        )
        return 1
    scanned = sum(1 for path in targets for _ in iter_files(path))
    summary = f"no leaks: {len(canaries)} canaries absent from {scanned} file(s)"
    if skipped:
        summary += f"; {len(skipped)} path(s) skipped"
    print(summary)
    return 0


def iter_files(path: Path):
    """Yield the files `scan` would read under `path`, for the summary count."""
    if path.is_file():
        yield path
    elif path.is_dir():
        skipped: list[tuple[Path, str]] = []
        yield from walk(path, skipped)


if __name__ == "__main__":
    raise SystemExit(main())
