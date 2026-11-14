#!/usr/bin/env python3
"""Publish the backlog issues to GitHub, in order, and record the CC-># mapping.

Usage:
    python3 scripts/publish_issues.py            # create labels + issues
    python3 scripts/publish_issues.py --labels   # only create labels

Requires: gh authenticated, backlog/issue_map.json absent (or delete it to republish).
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "backlog"))
from data_issues import EPICS, ISSUES  # noqa: E402

MAP = ROOT / "backlog" / "issue_map.json"

COLORS = {
    "P0": "b60205",
    "P1": "d93f0b",
    "P2": "fbca04",
    "P3": "c5def5",
    "feat": "0e8a16",
    "fix": "1d76db",
    "chore": "6e7781",
    "docs": "84b6eb",
    "test": "bfd4f2",
    "perf": "f9d0c4",
    "refactor": "c2e0c6",
}


def run(args: list[str], check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(args, check=check, capture_output=True, text=True)


def ensure_labels() -> None:
    labels: dict[str, str] = {}
    for e in EPICS:
        labels["epic:" + e["key"]] = "1e3a8a"
    for p, color in COLORS.items():
        labels[p] = color
    for issue in ISSUES:
        for lab in issue["labels"]:
            labels.setdefault(lab, "e2e4e8")

    for name, color in sorted(labels.items()):
        r = run(["gh", "label", "create", name, "--color", color, "--force"], check=False)
        if r.returncode == 0:
            print("label +", name)
        else:
            # Already exists is fine.
            pass


def body_for(issue: dict, epic: dict) -> str:
    lines = [
        "**Epic:** %s — %s  " % (epic["key"], epic["title"]),
        "**Type:** `%s`  **Priority:** %s  **Size:** %s  " % (issue["type"], issue["priority"], issue["size"]),
        "**Labels:** %s" % ", ".join("`%s`" % l for l in issue["labels"]),
        "",
        issue["body"],
        "",
        "**Acceptance criteria**",
        "",
    ]
    for c in issue["criteria"]:
        lines.append("- [ ] " + c)
    lines += [
        "- [ ] Unit tests cover the new logic and edge cases.",
        "- [ ] Docs updated (user, admin, or developer as appropriate).",
        "- [ ] CI green on macOS arm64/x64, Windows x64, and Linux x64.",
        "- [ ] No screen content or raw text appears in logs, dumps, or crash reports.",
    ]
    if issue["depends"]:
        lines += ["", "**Depends on:** " + ", ".join(issue["depends"])]
    lines += [
        "",
        "Generated from `backlog/data_issues.py`. Regenerate with `python3 scripts/render_backlog.py`.",
    ]
    return "\n".join(lines)


def main() -> int:
    if MAP.exists() and "--labels" not in sys.argv:
        print("issue_map.json already exists; delete it to republish", file=sys.stderr)
        return 2

    ensure_labels()
    if "--labels" in sys.argv:
        return 0

    epics = {e["key"]: e for e in EPICS}
    mapping: dict[str, int] = {}
    for issue in ISSUES:
        title = "[%s] %s" % (issue["id"], issue["title"])
        labels = ["epic:" + issue["epic"], issue["priority"], issue["type"]] + issue["labels"]
        args = ["gh", "issue", "create", "--title", title, "--body", body_for(issue, epics[issue["epic"]])]
        for lab in labels:
            args += ["--label", lab]
        r = run(args, check=False)
        if r.returncode != 0:
            print("FAILED", issue["id"], r.stderr.strip(), file=sys.stderr)
            return 1
        url = r.stdout.strip().splitlines()[-1]
        number = int(url.rstrip("/").split("/")[-1])
        mapping[issue["id"]] = number
        print("%s -> #%d" % (issue["id"], number))

    MAP.write_text(json.dumps(mapping, indent=2) + "\n", encoding="utf-8")
    print("wrote", MAP)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
