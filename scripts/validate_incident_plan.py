#!/usr/bin/env python3
"""Fail if the incident response plan loses a required guarantee.

The plan is prose, so nothing stops a well-meaning edit from quietly dropping
the 72-hour acknowledgement deadline or the "no screen content" rule. This
script re-reads the three documents and checks the facts that matter:

* ``docs/INCIDENT_RESPONSE.md`` defines all four severity levels, the
  containment levers, the disclosure deadlines, the escalation roles, the
  rehearsal cadence, and the content-free reporting rule.
* ``docs/POSTMORTEM_TEMPLATE.md`` keeps its impact, timeline, root-cause, and
  action-item sections, with owner and due-date columns on every action.
* ``backlog/INCIDENTS.md`` is the register, with a stable header.

Usage::

    python3 scripts/validate_incident_plan.py            # validate the repo
    python3 scripts/validate_incident_plan.py --self-test  # prove it fails

A document is never parsed as markdown structure beyond headings and table
rows; the checks are deliberately coarse so they survive rewording but still
catch a deleted guarantee. Missing files fail the run rather than passing
silently.
"""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / "docs" / "INCIDENT_RESPONSE.md"
TEMPLATE = ROOT / "docs" / "POSTMORTEM_TEMPLATE.md"
REGISTER = ROOT / "backlog" / "INCIDENTS.md"

SEVERITIES = ("SEV1", "SEV2", "SEV3", "SEV4")
CONTAINMENT_LEVERS = (
    "kill switch",
    "update channel",
    "rotate",
    "rollback",
)
ROLES = 5
HEADING_RE = re.compile(r"^#{1,6}\s+(.*)$", re.MULTILINE)


def headings(text: str) -> list[str]:
    return [m.group(1).strip().lower() for m in HEADING_RE.finditer(text)]


def has_heading(text: str, needle: str) -> bool:
    return any(needle in heading for heading in headings(text))


def table_rows(text: str) -> int:
    """Count markdown table body rows, ignoring the header and separator."""
    rows = 0
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped.startswith("|"):
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if all(re.fullmatch(r":?-{2,}:?", cell) for cell in cells):
            continue
        rows += 1
    return max(rows - 1, 0)


def validate_plan(text: str) -> list[str]:
    errors: list[str] = []
    for level in SEVERITIES:
        if not re.search(rf"^\|\s*{level}\s*\|", text, re.MULTILINE):
            errors.append(f"plan is missing the {level} severity row")
    lower = text.lower()
    for lever in CONTAINMENT_LEVERS:
        if lever not in lower:
            errors.append(f"plan is missing the containment lever {lever!r}")
    if not re.search(r"Acknowledge[^|\n]*\|\s*72 hours", text):
        errors.append("plan does not state the 72-hour acknowledgement deadline")
    if "postmortem within 5 business days" not in lower:
        errors.append("plan does not state the postmortem deadline")
    if not re.search(r"rehearsed\s+twice\s+a\s+year\s+and\s+after\s+any\s+SEV1", text):
        errors.append("plan does not state the rehearsal cadence")
    if "utc" not in lower:
        errors.append("plan does not require UTC timestamps")
    if "incident commander" not in lower:
        errors.append("plan does not name an Incident Commander role")
    if not has_heading(text, "rehearsal"):
        errors.append("plan has no rehearsal section")
    if table_rows(text) < ROLES:
        errors.append(f"plan has fewer than {ROLES} role or severity table rows")
    # The content-free rule must be stated in both directions: what is kept
    # (counts/hashes) and what is forbidden (screen content).
    if "report counts and hashes only" not in lower:
        errors.append(
            "plan does not state that incident reports carry counts and hashes only"
        )
    if "screen content" not in lower or "ocr text" not in lower:
        errors.append("plan does not forbid screen content and OCR text in artifacts")
    return errors


def validate_template(text: str) -> list[str]:
    errors: list[str] = []
    required = {
        "summary": "Summary",
        "impact": "Impact",
        "timeline": "Timeline",
        "root cause": "Root cause",
        "action items": "Action items",
        "evidence": "Evidence",
    }
    for needle, label in required.items():
        if not has_heading(text, needle):
            errors.append(f"postmortem template is missing the {label} section")
    upper = text.upper()
    if "REAL" not in upper or "DRY RUN" not in upper:
        errors.append("postmortem template does not distinguish REAL from DRY RUN")
    if not any(
        line.strip().startswith("|")
        and "action" in line.lower()
        and "owner" in line.lower()
        and "due" in line.lower()
        for line in text.splitlines()
    ):
        errors.append("postmortem action items lack owner and due-date columns")
    if "counts and hashes" not in text.lower():
        errors.append("postmortem template does not restrict evidence to counts and hashes")
    return errors


REGISTER_COLUMNS = ("date", "incident", "severity", "status", "type", "link")


def validate_register(text: str) -> list[str]:
    errors: list[str] = []
    header: list[str] | None = None
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("|") and "date" in stripped.lower():
            header = [cell.strip().lower() for cell in stripped.strip("|").split("|")]
            break
    if header is None:
        errors.append("incident register has no incidents table")
    else:
        for column in REGISTER_COLUMNS:
            if column not in header:
                errors.append(f"incident register is missing the {column!r} column")
    lowered = text.lower()
    if "real" not in lowered or "dry run" not in lowered:
        errors.append("incident register does not explain the REAL/DRY RUN marker")
    return errors


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    for path in (PLAN, TEMPLATE, REGISTER):
        if not (root / path.relative_to(ROOT)).exists():
            errors.append(f"missing document: {path.relative_to(ROOT)}")
    if errors:
        return errors
    errors.extend(
        f"{PLAN.relative_to(ROOT)}: {message}"
        for message in validate_plan((root / PLAN.relative_to(ROOT)).read_text(encoding="utf-8"))
    )
    errors.extend(
        f"{TEMPLATE.relative_to(ROOT)}: {message}"
        for message in validate_template(
            (root / TEMPLATE.relative_to(ROOT)).read_text(encoding="utf-8")
        )
    )
    errors.extend(
        f"{REGISTER.relative_to(ROOT)}: {message}"
        for message in validate_register(
            (root / REGISTER.relative_to(ROOT)).read_text(encoding="utf-8")
        )
    )
    return errors


def _mutated_copy(root: Path) -> Path:
    """Copy the three documents into a temporary tree for the self-test."""
    tmp = Path(tempfile.mkdtemp(prefix="curbcut-incident-selftest-"))
    (tmp / "docs").mkdir()
    (tmp / "backlog").mkdir()
    for source in (PLAN, TEMPLATE, REGISTER):
        target = tmp / source.relative_to(ROOT)
        target.write_text(source.read_text(encoding="utf-8"), encoding="utf-8")
    return tmp


def _break(document: Path, old: str, new: str) -> None:
    text = document.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"self-test fixture no longer matches {document.name}: {old!r}")
    document.write_text(text.replace(old, new, 1), encoding="utf-8")


def self_test() -> int:
    root = _mutated_copy(ROOT)
    try:
        if validate(root):
            print("SELF-TEST FAILED: a pristine copy did not pass", file=sys.stderr)
            return 1
        print("self-test: the committed documents pass")

        cases: list[tuple[Path, str, str, str]] = [
            (root / "docs" / "INCIDENT_RESPONSE.md", "| SEV3 |", "| -- |", "a severity row"),
            (
                root / "docs" / "INCIDENT_RESPONSE.md",
                "| Acknowledge a reporter | 72 hours | Comms Lead |",
                "| Acknowledge a reporter | soon | Comms Lead |",
                "the disclosure deadline",
            ),
            (
                root / "docs" / "INCIDENT_RESPONSE.md",
                "Report counts and hashes only.",
                "Report whatever is useful.",
                "the content-free rule",
            ),
            (
                root / "docs" / "INCIDENT_RESPONSE.md",
                "The plan is rehearsed twice a year and after any SEV1 or SEV2",
                "The plan is reviewed when someone remembers",
                "the rehearsal cadence",
            ),
            (
                root / "docs" / "INCIDENT_RESPONSE.md",
                "## 8. Rehearsal",
                "## 8. Notes",
                "the rehearsal section",
            ),
            (
                root / "docs" / "POSTMORTEM_TEMPLATE.md",
                "## Root cause",
                "## Analysis",
                "the root-cause section",
            ),
            (
                root / "docs" / "POSTMORTEM_TEMPLATE.md",
                "| ID | Action | Owner | Due | Kind |",
                "| ID | Action | Kind |",
                "the action-item columns",
            ),
            (
                root / "backlog" / "INCIDENTS.md",
                "| Date | Incident | Severity | Status | Type | Link |",
                "| Date | Incident | Type | Link |",
                "the register columns",
            ),
        ]
        for path, old, new, label in cases:
            _break(path, old, new)
            errors = validate(root)
            if not errors:
                print(
                    f"SELF-TEST FAILED: removing {label} was not detected "
                    f"({path.name})",
                    file=sys.stderr,
                )
                return 1
            _break(path, new, old)

        # A missing document must fail, not pass.
        (root / "docs" / "POSTMORTEM_TEMPLATE.md").unlink()
        if not validate(root):
            print("SELF-TEST FAILED: a missing document was not reported", file=sys.stderr)
            return 1

        print(f"self-test: {len(cases)} mutations were each detected, and a missing document failed")
        return 0
    finally:
        for path in sorted(root.rglob("*"), reverse=True):
            if path.is_file():
                path.unlink()
            elif path.is_dir():
                path.rmdir()
        root.rmdir()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="mutate copies of the documents and require detection",
    )
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    errors = validate(ROOT)
    for message in errors:
        print(f"FAIL: {message}", file=sys.stderr)
    if errors:
        print(
            f"{len(errors)} incident-plan check(s) failed",
            file=sys.stderr,
        )
        return 1
    print("incident plan, postmortem template, and register are complete")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
