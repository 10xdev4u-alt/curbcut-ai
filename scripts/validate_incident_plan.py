#!/usr/bin/env python3
"""Fail if the incident response plan loses a required guarantee.

The plan is prose, so nothing stops a well-meaning edit from quietly dropping
the 72-hour acknowledgement deadline or the "never attach a frame" rule. This
script re-reads three documents and checks the facts that matter:

* ``docs/INCIDENT_RESPONSE.md`` defines all four severity levels, the
  containment levers, the disclosure deadlines, the escalation roles, the
  rehearsal cadence, and the content-free reporting rule.
* ``docs/POSTMORTEM_TEMPLATE.md`` keeps its impact, timeline, root-cause, and
  action-item sections, with owner and due-date columns on every action.
* ``backlog/INCIDENTS.md`` is the register, with a stable header.

Every check is scoped to the section that carries its fact. A lever counts only
as a row in the containment table; a deadline only as a row in the disclosure
table. A whole-document substring test passes when the same word survives in
prose elsewhere, which is exactly how a deleted guarantee used to slip through.

Usage::

    python3 scripts/validate_incident_plan.py            # validate the repo
    python3 scripts/validate_incident_plan.py --self-test  # prove it fails

``--self-test`` copies the three documents into a temporary tree, applies one
named deletion per guarantee, and requires each to be detected. It fails if a
document is missing. Mutations target the line that carries the fact, so the
copy always returns to a passing state before the next case. Some mutations
trip more than one check (removing a table header drops every column), so the
mutation count is deliberately not one-to-one with the number of checks.
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
ROLES = 5
HEADING_RE = re.compile(r"^(#{1,6})\s+(.*)$", re.MULTILINE)

# The containment steps that must appear as rows in section 4.
CONTAINMENT_STEPS = (
    "stop capture with the local panic key",
    "network kill switch",
    "halt the update channel",
    "revoke and rotate signing keys",
    "rotate store/cache keys",
    "roll back to the last known-good release",
    "disable the optional network features",
)

TEMPLATE_SECTIONS = {
    "summary": "Summary",
    "impact": "Impact",
    "timeline": "Timeline",
    "root cause": "Root cause",
    "action items": "Action items",
    "evidence": "Evidence",
}

REGISTER_COLUMNS = ("date", "incident", "severity", "status", "type", "link")


def headings(text: str) -> list[str]:
    return [m.group(2).strip().lower() for m in HEADING_RE.finditer(text)]


def has_heading(text: str, needle: str) -> bool:
    return any(needle.lower() in heading for heading in headings(text))


def section(text: str, needle: str) -> str:
    """The body of the first section whose heading contains ``needle``.

    The section ends at the next heading of the same or a higher level, so a
    sub-heading inside it stays part of the body.
    """
    lines = text.splitlines()
    start = None
    level = 0
    for index, line in enumerate(lines):
        match = HEADING_RE.match(line)
        if match and needle.lower() in match.group(2).lower():
            start = index + 1
            level = len(match.group(1))
            break
    if start is None:
        return ""
    end = len(lines)
    for index in range(start, len(lines)):
        match = HEADING_RE.match(lines[index])
        if match and len(match.group(1)) <= level:
            end = index
            break
    return "\n".join(lines[start:end])


def table_rows(text: str) -> list[str]:
    """The markdown table rows in ``text``, header included, separator excluded."""
    rows: list[str] = []
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped.startswith("|"):
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if all(re.fullmatch(r":?-{2,}:?", cell) for cell in cells):
            continue
        rows.append(stripped)
    return rows


def validate_plan(text: str) -> list[str]:
    errors: list[str] = []

    severity = section(text, "Severity levels")
    for level in SEVERITIES:
        if not any(
            re.match(rf"\|\s*{level}\s*\|", row) for row in table_rows(severity)
        ):
            errors.append(f"plan is missing the {level} severity row")

    roles = section(text, "Roles and escalation")
    role_rows = table_rows(roles)
    # `table_rows` keeps the header, so the body is one row shorter. Counting the
    # header as a role let a deleted role pass.
    if len(role_rows) - 1 < ROLES:
        errors.append(f"plan has fewer than {ROLES} role table rows")
    # Match the role's own row, not a mention in another role's backup column.
    if not any(re.match(r"\|\s*incident commander\s*\|", row.lower()) for row in role_rows):
        errors.append("plan does not name an Incident Commander role")

    containment = section(text, "Containment")
    containment_rows = table_rows(containment)
    lowered_rows = [row.lower() for row in containment_rows]
    for step in CONTAINMENT_STEPS:
        if not any(step in row for row in lowered_rows):
            errors.append(f"plan is missing the containment step {step!r}")
    # Require the normative claim and its reason. Matching only "no remote
    # control" let a sentence like "... but a server console is planned" pass.
    containment_lower = containment.lower()
    if (
        "there is no remote control of an installed copy" not in containment_lower
        or "cannot be reached from a server" not in containment_lower
    ):
        errors.append(
            "plan does not state that an installed copy cannot be controlled remotely"
        )
    # The step names are not enough: an earlier revision kept the right name and
    # claimed it removed the update path, which NETWORK_POLICY.md says survives
    # in every build. Check the effect this row actually promises.
    feature_row = next(
        (row for row in lowered_rows if "disable the optional network features" in row),
        "",
    )
    if "update checks stay behind the kill switch" not in feature_row:
        errors.append(
            "plan does not state that update checks stay behind the kill switch"
        )

    disclosure = section(text, "Disclosure")
    disclosure_rows = table_rows(disclosure)
    if not any(
        "acknowledge" in row.lower() and "72 hours" in row.lower()
        for row in disclosure_rows
    ):
        errors.append("plan does not state the 72-hour acknowledgement deadline")
    if not any(
        "triage" in row.lower() and "5 business days" in row.lower()
        for row in disclosure_rows
    ):
        errors.append("plan does not state the triage deadline")
    if not any("90 days" in row for row in disclosure_rows):
        errors.append("plan does not state the 90-day advisory promise")
    if not any(
        "supervisory authority" in row.lower() and "72 hours" in row.lower()
        for row in disclosure_rows
    ):
        errors.append("plan does not state the GDPR notification deadline")
    if not any(
        "notify affected users" in row.lower() and "without undue delay" in row.lower()
        for row in disclosure_rows
    ):
        errors.append("plan does not state the user-notification deadline")

    evidence = section(text, "Evidence handling").lower()
    if "never attach a frame" not in evidence or "ocr text" not in evidence:
        errors.append("plan does not forbid frames and OCR text in evidence")
    if "counts" not in evidence or "hashes" not in evidence:
        errors.append("plan does not limit evidence to counts and hashes")
    if "18 months" not in evidence:
        errors.append("plan does not state the evidence retention period")
    if "utc" not in evidence:
        errors.append("plan does not require UTC timestamps")

    rehearsal = section(text, "Rehearsal")
    if not re.search(
        r"rehearsed\s+twice\s+a\s+year\s+and\s+after\s+any\s+SEV1", rehearsal
    ):
        errors.append("plan does not state the rehearsal cadence")
    if "checklist" not in rehearsal.lower():
        errors.append("plan has no rehearsal checklist")

    return errors


def validate_template(text: str) -> list[str]:
    errors: list[str] = []
    for needle, label in TEMPLATE_SECTIONS.items():
        if not has_heading(text, needle):
            errors.append(f"postmortem template is missing the {label} section")
    # Check the `Type:` line itself. Searching the whole document let the prose
    # "(dry runs only)" satisfy the marker after `DRY RUN` was deleted from it.
    type_line = next(
        (line for line in text.splitlines() if line.lower().startswith("type:")),
        "",
    )
    if "REAL" not in type_line.upper() or "DRY RUN" not in type_line.upper():
        errors.append("postmortem template does not distinguish REAL from DRY RUN")
    if not any(
        row.strip().startswith("|")
        and "action" in row.lower()
        and "owner" in row.lower()
        and "due" in row.lower()
        for row in text.splitlines()
    ):
        errors.append("postmortem action items lack owner and due-date columns")
    evidence = section(text, "Evidence").lower()
    # The normative phrase, not the words: the retention bullet also mentions
    # "redacted excerpts", so a looser test survives deleting the rule.
    if "redacted excerpts only" not in evidence:
        errors.append("postmortem evidence section does not require redaction")
    if "never retain an unredacted original" not in evidence:
        errors.append("postmortem evidence section permits unredacted originals")
    return errors


def validate_register(text: str) -> list[str]:
    errors: list[str] = []
    # Identify the header by structure: the first table row followed by a
    # separator. Searching for the word "date" made that column undetectable,
    # because renaming it hid the whole table behind "no incidents table".
    lines = text.splitlines()
    header: list[str] | None = None
    for index, line in enumerate(lines):
        stripped = line.strip()
        if not stripped.startswith("|"):
            continue
        following = lines[index + 1].strip() if index + 1 < len(lines) else ""
        if not following.startswith("|"):
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if all(re.fullmatch(r":?-{2,}:?", cell) for cell in cells):
            continue
        header = [cell.lower() for cell in cells]
        break
    if header is None:
        errors.append("incident register has no incidents table")
    else:
        for column in REGISTER_COLUMNS:
            if column not in header:
                errors.append(f"incident register is missing the {column!r} column")
    # Require the normative sentence, not the two words: they also appear in the
    # status vocabulary above the table.
    lowered = text.lower()
    if "rehearsals must say so" not in lowered:
        errors.append("incident register does not require rehearsals to be marked")
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


# One named mutation per guarantee, in document order. Every entry must be
# detected by `--self-test`; several trip more than one check, so this is a
# floor on coverage rather than an exact map onto the checks above.
MUTATIONS: tuple[tuple[str, str, str, str], ...] = (
    ("sev1 row", "docs/INCIDENT_RESPONSE.md", "| SEV1 | Active exfiltration", "| -- | Active exfiltration"),
    ("sev2 row", "docs/INCIDENT_RESPONSE.md", "| SEV2 | Exposure", "| -- | Exposure"),
    ("sev3 row", "docs/INCIDENT_RESPONSE.md", "| SEV3 | A feature", "| -- | A feature"),
    ("sev4 row", "docs/INCIDENT_RESPONSE.md", "| SEV4 | Cosmetic", "| -- | Cosmetic"),
    ("ic role", "docs/INCIDENT_RESPONSE.md", "| Incident Commander | Declares severity", "| -- | Declares severity"),
    (
        "ops role",
        "docs/INCIDENT_RESPONSE.md",
        "| Ops Lead | Runs the technical steps: network kill switch, capture stop, update halt, rollback, key rotation | Platform maintainer | Any maintainer |",
        "",
    ),
    ("capture stop step", "docs/INCIDENT_RESPONSE.md", "| Stop capture with the local panic key |", "| -- |"),
    ("network switch step", "docs/INCIDENT_RESPONSE.md", "| Engage the network kill switch", "| -- "),
    ("update halt step", "docs/INCIDENT_RESPONSE.md", "| Halt the update channel |", "| -- |"),
    ("signing key step", "docs/INCIDENT_RESPONSE.md", "| Revoke and rotate signing keys |", "| -- |"),
    ("store key step", "docs/INCIDENT_RESPONSE.md", "| Rotate store/cache keys and force re-auth |", "| -- |"),
    ("rollback step", "docs/INCIDENT_RESPONSE.md", "| Roll back to the last known-good release |", "| -- |"),
    ("feature disable step", "docs/INCIDENT_RESPONSE.md", "| Disable the optional network features at the source |", "| -- |"),
    ("remote-control limit", "docs/INCIDENT_RESPONSE.md", "**There is no remote control of an installed copy.**", "**Remote control is available.**"),
    ("feature egress effect", "docs/INCIDENT_RESPONSE.md", "update checks stay behind the kill switch", "update checks are always online"),
    ("acknowledgement deadline", "docs/INCIDENT_RESPONSE.md", "| Acknowledge a reporter | 72 hours | Comms Lead |", "| Acknowledge a reporter | soon | Comms Lead |"),
    ("triage deadline", "docs/INCIDENT_RESPONSE.md", "| Triage and confirm or reject | 5 business days | IC |", "| Triage and confirm or reject | when possible | IC |"),
    ("advisory promise", "docs/INCIDENT_RESPONSE.md", "or 90 days after acknowledgement", "when convenient"),
    ("gdpr deadline", "docs/INCIDENT_RESPONSE.md", "| Notify a supervisory authority, where GDPR applies | 72 hours from becoming aware | Legal contact |", "| Notify a supervisory authority, where GDPR applies | without undue delay | Legal contact |"),
    ("user notice deadline", "docs/INCIDENT_RESPONSE.md", "| Notify affected users, where required | Without undue delay after the authority | Legal contact |", "| Notify affected users, where required | eventually | Legal contact |"),
    ("frame prohibition", "docs/INCIDENT_RESPONSE.md", "Never attach a frame, a screenshot of content, OCR text, or a raw", "Attach whatever helps, including a frame or OCR text or a raw"),
    ("counts and hashes", "docs/INCIDENT_RESPONSE.md", "counts, lengths, dimensions,", "details,"),
    ("retention period", "docs/INCIDENT_RESPONSE.md", "Retain evidence for 18 months, then delete it.", "Keep evidence indefinitely."),
    ("utc timestamps", "docs/INCIDENT_RESPONSE.md", "with timestamps in UTC.", "with local timestamps."),
    ("rehearsal cadence", "docs/INCIDENT_RESPONSE.md", "The plan is rehearsed twice a year and after any SEV1 or SEV2", "The plan is reviewed when needed"),
    ("rehearsal checklist", "docs/INCIDENT_RESPONSE.md", "Checklist:", "Notes:"),
    ("template summary", "docs/POSTMORTEM_TEMPLATE.md", "## Summary", "## Overview"),
    ("template impact", "docs/POSTMORTEM_TEMPLATE.md", "## Impact", "## Effects"),
    ("template timeline", "docs/POSTMORTEM_TEMPLATE.md", "## Timeline (UTC)", "## When"),
    ("template root cause", "docs/POSTMORTEM_TEMPLATE.md", "## Root cause", "## Analysis"),
    ("template action items", "docs/POSTMORTEM_TEMPLATE.md", "## Action items", "## Follow-ups"),
    ("template evidence", "docs/POSTMORTEM_TEMPLATE.md", "## Evidence", "## Attachments"),
    ("template action columns", "docs/POSTMORTEM_TEMPLATE.md", "| ID | Action | Owner | Due | Kind |", "| ID | Action | Kind |"),
    ("template status marker", "docs/POSTMORTEM_TEMPLATE.md", "Type: `REAL | DRY RUN`", "Type: `SOMETIMES`"),
    ("template redaction", "docs/POSTMORTEM_TEMPLATE.md", "Redacted excerpts only, from the categories", "Excerpts of any kind, from any source,"),
    ("template originals", "docs/POSTMORTEM_TEMPLATE.md", "Never retain an unredacted original.", "Originals may be retained."),
    ("register header", "backlog/INCIDENTS.md", "| Date | Incident | Severity | Status | Type | Link |\n", ""),
    ("register date column", "backlog/INCIDENTS.md", "| Date | Incident | Severity | Status | Type | Link |", "| Incident | Severity | Status | Type | Link |"),
    ("register other columns", "backlog/INCIDENTS.md", "| Date | Incident | Severity | Status | Type | Link |", "| Date |"),
    ("register marker", "backlog/INCIDENTS.md", "`DRY RUN`; rehearsals must say so.", "`MAYBE`."),
)


def self_test() -> int:
    root = _mutated_copy(ROOT)
    try:
        if validate(root):
            print("SELF-TEST FAILED: a pristine copy did not pass", file=sys.stderr)
            return 1
        print("self-test: the committed documents pass")

        for label, relative, old, new in MUTATIONS:
            document = root / relative
            _break(document, old, new)
            errors = validate(root)
            if not errors:
                print(
                    f"SELF-TEST FAILED: removing {label} was not detected "
                    f"({Path(relative).name})",
                    file=sys.stderr,
                )
                return 1
            _break(document, new, old)

        # A missing document must fail, not pass.
        (root / TEMPLATE.relative_to(ROOT)).unlink()
        if not validate(root):
            print("SELF-TEST FAILED: a missing document was not reported", file=sys.stderr)
            return 1

        print(
            f"self-test: {len(MUTATIONS)} mutations were each detected, "
            "and a missing document failed"
        )
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
        help=f"apply {len(MUTATIONS)} named deletions and require detection",
    )
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    errors = validate(ROOT)
    for message in errors:
        print(f"FAIL: {message}", file=sys.stderr)
    if errors:
        print(f"{len(errors)} incident-plan check(s) failed", file=sys.stderr)
        return 1
    print("incident plan, postmortem template, and register are complete")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
