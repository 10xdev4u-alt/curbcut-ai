#!/usr/bin/env python3
"""Fail when a command opens a network connection.

The offline build must make zero outbound connections. This guard runs a
command under `strace` on Linux and fails if the command calls `connect(2)` on
anything but a Unix socket. Unix sockets are not network connections, so they
are ignored.

Usage:
    python3 scripts/socket_guard.py COMMAND [ARGS...]
    python3 scripts/socket_guard.py --self-test

Exit codes:
    0  no network connection was attempted, or the self-test passed
    1  a network connection was attempted, or the self-test failed
    2  the guard could not run (strace missing, or not Linux)
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile

# A loopback connection that is refused, so the self-test never reaches a
# service. The `connect(2)` call still appears in the trace.
LOOPBACK_CONNECT = (
    "import socket\n"
    "probe = socket.socket()\n"
    "try:\n"
    "    probe.connect(('127.0.0.1', 9))\n"
    "except OSError:\n"
    "    pass\n"
    "probe.close()\n"
)


class GuardError(RuntimeError):
    """The guard could not run."""


def watched(command: list[str]) -> tuple[list[str], int]:
    """Runs `command` under strace and returns its network connects and exit code."""
    if not sys.platform.startswith("linux"):
        raise GuardError("the socket guard is Linux-only (it uses strace)")
    strace = shutil.which("strace")
    if strace is None:
        raise GuardError("strace is not installed")

    handle = tempfile.NamedTemporaryFile("r", suffix=".strace", delete=False)
    log_path = handle.name
    handle.close()
    try:
        traced = subprocess.run(
            [strace, "-f", "-e", "trace=connect", "-o", log_path, "--", *command],
            capture_output=True,
            text=True,
        )
        with open(log_path, encoding="utf-8", errors="replace") as log:
            lines = log.read().splitlines()
    finally:
        os.unlink(log_path)

    connects = [
        line.strip()
        for line in lines
        if "connect(" in line and "sa_family=AF_UNIX" not in line
    ]
    return connects, traced.returncode


def guard(command: list[str]) -> int:
    """Fails when `command` opens a network connection."""
    try:
        connects, status = watched(command)
    except GuardError as error:
        print(f"socket guard: {error}", file=sys.stderr)
        return 2
    for line in connects:
        print(f"socket guard: network connect: {line}", file=sys.stderr)
    if connects:
        print(
            f"socket guard: {len(connects)} network connect(s) attempted",
            file=sys.stderr,
        )
        return 1
    if status != 0:
        print(f"socket guard: command exited {status}", file=sys.stderr)
        return 1
    print("socket guard: no network connection attempted")
    return 0


def self_test() -> int:
    """Proves the guard observes a connection, so a clean run is not vacuous."""
    try:
        connects, status = watched([sys.executable, "-c", LOOPBACK_CONNECT])
    except GuardError as error:
        print(f"socket guard self-test: {error}", file=sys.stderr)
        return 2
    if not connects:
        print(
            "socket guard self-test: a loopback connect was not observed; "
            "the guard would pass anything",
            file=sys.stderr,
        )
        return 1
    if status != 0:
        print(f"socket guard self-test: probe exited {status}", file=sys.stderr)
        return 1
    print(f"socket guard self-test: observed {len(connects)} connect(s)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", nargs=argparse.REMAINDER, help="the command to watch")
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="prove the guard observes a connection, then exit",
    )
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    command = args.command
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        parser.error("a command is required")
    return guard(command)


if __name__ == "__main__":
    raise SystemExit(main())
