#!/usr/bin/env python3
"""Render ``docs/ROLE_MAPPING.md`` from the mapping tables.

``ROLE_COMPAT`` and ``STATE_COMPAT`` in ``crates/curbcut-a11y/src/mapping.rs``
are the source of truth. This script extracts their rows and writes the
Markdown tables, so the doc cannot drift from the code.

Usage:

    python3 scripts/render_role_mapping.py          # write the doc
    python3 scripts/render_role_mapping.py --check  # fail if it is stale
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "crates" / "curbcut-a11y" / "src" / "mapping.rs"
TARGET = ROOT / "docs" / "ROLE_MAPPING.md"

PLATFORM_ORDER = ["MacOs", "Windows", "Linux"]
PLATFORM_TITLES = {"MacOs": "macOS", "Windows": "Windows", "Linux": "Linux"}

ROLE_RE = re.compile(
    r"RoleMapping\s*\{.*?platform:\s*Platform::(\w+),\s*"
    r'raw:\s*"([^"]+)",\s*core:\s*Role::(\w+)',
    re.DOTALL,
)
STATE_RE = re.compile(
    r"StateMapping\s*\{.*?platform:\s*Platform::(\w+),\s*"
    r'raw:\s*"([^"]+)",\s*flag:\s*StateFlag::(\w+),\s*value:\s*(true|false)',
    re.DOTALL,
)


def parse_rows(source: str) -> tuple[list[tuple[str, str, str]], list[tuple[str, str, str, str]]]:
    roles = [(platform, raw, core) for platform, raw, core in ROLE_RE.findall(source)]
    states = [
        (platform, raw, flag, value)
        for platform, raw, flag, value in STATE_RE.findall(source)
    ]
    if not roles or not states:
        raise SystemExit(f"no mapping rows found in {SOURCE}")
    return roles, states


def render() -> str:
    roles, states = parse_rows(SOURCE.read_text(encoding="utf-8"))
    out: list[str] = []
    out.append("# Role and state mapping")
    out.append("")
    out.append(
        "> How the three platform accessibility APIs map onto the core `Role` and"
    )
    out.append(
        "> `ElementState`. The tables below are rendered from `ROLE_COMPAT` and"
    )
    out.append(
        "> `STATE_COMPAT` in `crates/curbcut-a11y/src/mapping.rs` by"
    )
    out.append(
        "> `scripts/render_role_mapping.py`; the tests fail if a row stops mapping"
    )
    out.append("> to a concrete role.")
    out.append("")
    out.append("## Raw role vocabularies")
    out.append("")
    out.append("| Platform | Raw string source | Example |")
    out.append("| --- | --- | --- |")
    out.append("| macOS | `AXRole` attribute | `AXButton` |")
    out.append("| Windows | UIA `ControlType` name | `Button` |")
    out.append("| Linux | AT-SPI role name | `push-button` |")
    out.append("")
    out.append("## Role compatibility table")
    for platform in PLATFORM_ORDER:
        out.append("")
        out.append(f"### {PLATFORM_TITLES[platform]}")
        out.append("")
        out.append("| Raw role | Core role |")
        out.append("| --- | --- |")
        for row_platform, raw, core in roles:
            if row_platform == platform:
                out.append(f"| `{raw}` | `{core}` |")
    out.append("")
    out.append("## Unmapped roles")
    out.append("")
    out.append("A raw role absent from the table becomes `Role::Unknown(raw)`. The raw")
    out.append("string is preserved, so an unmapped role is flagged in the provenance")
    out.append("panel rather than dropped. Container chrome with no core equivalent")
    out.append("(for example `AXScrollBar`, `ProgressBar`, `scroll-bar`) is deliberately")
    out.append("left unmapped.")
    out.append("")
    out.append("## State mapping")
    out.append("")
    out.append(
        "`map_state` starts from `Platform::default_enabled`: macOS and Windows are"
    )
    out.append(
        "enabled unless they say otherwise, while AT-SPI omits `enabled` for a"
    )
    out.append(
        "disabled element, so Linux starts disabled. A name the table does not know"
    )
    out.append(
        "is ignored, so a new platform state cannot corrupt an element. When a"
    )
    out.append(
        "platform reports contradictory names, the last name in `states` wins."
    )
    for platform in PLATFORM_ORDER:
        out.append("")
        out.append(f"### {PLATFORM_TITLES[platform]}")
        out.append("")
        out.append("| Raw state | Flag | Value |")
        out.append("| --- | --- | --- |")
        for row_platform, raw, flag, value in states:
            if row_platform == platform:
                out.append(f"| `{raw}` | `{flag}` | `{value}` |")
    out.append("")
    out.append("## Adding a platform role")
    out.append("")
    out.append("1. Add a `RoleMapping` row to `ROLE_COMPAT`.")
    out.append("2. Add the raw string to the platform adapter's test fixture.")
    out.append("3. Run `python3 scripts/render_role_mapping.py`.")
    out.append("")
    return "\n".join(out)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit 1 instead of writing when the doc is stale",
    )
    args = parser.parse_args()

    rendered = render()
    if args.check:
        current = TARGET.read_text(encoding="utf-8")
        if current != rendered:
            print(f"{TARGET} is stale; run scripts/render_role_mapping.py", file=sys.stderr)
            return 1
        print(f"{TARGET} is current")
        return 0

    TARGET.write_text(rendered, encoding="utf-8")
    print(f"wrote {TARGET}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
