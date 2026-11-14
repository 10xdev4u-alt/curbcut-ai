#!/usr/bin/env python3
"""Scaffold a correctly-labelled issue into backlog/data_issues.py.

Example:
    python3 scripts/new_issue.py \
        --epic E03 --title "Wayland fractional-scale capture fix" \
        --type fix --priority P1 --size M \
        --labels capture,linux \
        --body "At 125%/150% scale the overlay rects drift by one device pixel." \
        --criteria "Rects align within 1px at 100/125/150/175/200%." "A mixed-DPI test passes."

Then run:  python3 scripts/render_backlog.py
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "backlog" / "data_issues.py"

VALID_TYPES = {"feat", "fix", "chore", "docs", "test", "perf", "refactor"}
VALID_PRIORITIES = {"P0", "P1", "P2", "P3"}
VALID_SIZES = {"S", "M", "L", "XL"}


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Add an issue to the CurbCut backlog.")
    p.add_argument("--epic", required=True, help="Epic key, e.g. E03")
    p.add_argument("--title", required=True)
    p.add_argument("--type", dest="type_", required=True, choices=sorted(VALID_TYPES))
    p.add_argument("--priority", required=True, choices=sorted(VALID_PRIORITIES))
    p.add_argument("--size", required=True, choices=sorted(VALID_SIZES))
    p.add_argument("--labels", required=True, help="Comma-separated labels")
    p.add_argument("--body", required=True)
    p.add_argument("--criteria", nargs="+", required=True)
    p.add_argument("--depends", nargs="*", default=[])
    p.add_argument("--dry-run", action="store_true")
    return p.parse_args()


def main() -> int:
    args = parse_args()
    text = DATA.read_text(encoding="utf-8")

    epics = re.findall(r'"key": "(E\d+)"', text)
    if args.epic not in epics:
        print("error: unknown epic %s (have %s)" % (args.epic, ", ".join(epics)), file=sys.stderr)
        return 2

    ids = [int(n) for n in re.findall(r'I\([\'"]CC-(\d+)[\'"]', text)]
    if not ids:
        print("error: no issues found in data file", file=sys.stderr)
        return 2
    new_id = "CC-%03d" % (max(ids) + 1)

    labels = [l.strip() for l in args.labels.split(",") if l.strip()]
    crits = list(args.criteria)
    depends = list(args.depends)

    block = [
        '    I(%r, %r, %r, %r, %r, %r,' % (
            new_id, args.epic, args.title, args.type_, args.priority, args.size),
        '      %r,' % (labels,),
        '      %r,' % (args.body,),
        '      %r),' % (crits,),
    ]
    if depends:
        block.append('      depends=%r,' % (depends,))
    entry = "\n".join(block) + "\n"

    # Insert before the closing bracket of the ISSUES list.
    stripped = text.rstrip()
    idx = stripped.rfind("\n]")
    if idx == -1:
        print("error: could not find end of ISSUES list", file=sys.stderr)
        return 2

    new_text = text[: idx + 1] + entry + text[idx + 1 :]
    if args.dry_run:
        print(entry, end="")
        return 0

    DATA.write_text(new_text, encoding="utf-8")
    print("added %s to %s" % (new_id, DATA.relative_to(ROOT)))
    print("next: python3 scripts/render_backlog.py")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
