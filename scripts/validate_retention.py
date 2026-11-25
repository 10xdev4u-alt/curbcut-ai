#!/usr/bin/env python3
"""Fail if the retention reconciliation drifts back to a day-count promise.

Issue #200 found two current-looking rules (7-day logs, 30-day action log)
that no code enforces. `docs/DATA_INVENTORY.md` §Retention decisions is now
canonical: each class has one enforced rule, and every day count is labelled a
planned target. This script checks that the section, its consumers, the
findings register, and the code-level bounds still agree.

Run ``--self-test`` to apply every named mutation to a pristine copy and
require detection.
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

INVENTORY = "docs/DATA_INVENTORY.md"
ARCHITECTURE = "docs/ARCHITECTURE.md"
THREAT = "docs/THREAT_MODEL.md"
CONFIRM = "docs/CONFIRM.md"
WRITE_GUARD = "docs/WRITE_GUARD.md"
REDACTION = "docs/REDACTION.md"
DPIA = "docs/DPIA.md"
FINDINGS = "docs/PRIVACY_FINDINGS.md"
CONFIRM_RS = "crates/curbcut-adapt/src/confirm.rs"
PROVENANCE_RS = "crates/curbcut-adapt/src/provenance.rs"
WRITE_GUARD_RS = "crates/curbcut-a11y/src/write_guard.rs"
REDACT_RS = "crates/curbcut-core/src/redact.rs"
README = "README.md"
WORKFLOW = ".github/workflows/ci.yml"

DOCUMENTS = (
    INVENTORY,
    ARCHITECTURE,
    THREAT,
    CONFIRM,
    WRITE_GUARD,
    REDACTION,
    DPIA,
    FINDINGS,
    CONFIRM_RS,
    PROVENANCE_RS,
    WRITE_GUARD_RS,
    REDACT_RS,
    README,
    WORKFLOW,
)

DECISION_HEADER = "| Class | Inventory rule | Enforced today | Planned target | Finding |"
DECISIONS = {
    "DC-20": ("session", "changed_at_ms is stored but never expires"),
    "DC-21": ("session", "no timestamp and no purge"),
    "DC-22": ("bounded-records", "max_retained_records = 4096"),
    "DC-26": ("7-days", "no disk sink and no expiry"),
}
CLOCK_PRIMITIVE = re.compile(r"Instant|SystemTime|Duration|std::time|sleep|expire after")


def flatten(text: str) -> str:
    return " ".join(text.lower().split())


def table_cells(text: str, first_cell: str) -> list[str] | None:
    for line in text.splitlines():
        stripped = line.strip()
        if not (stripped.startswith("|") and stripped.endswith("|")):
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if cells and cells[0] == first_cell:
            return cells
    return None


def decision_cells(text: str, dc: str) -> list[str] | None:
    for line in text.splitlines():
        stripped = line.strip()
        if not (stripped.startswith("|") and stripped.endswith("|")):
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if cells and f"({dc})" in cells[0]:
            return cells
    return None


def row_containing(text: str, needle: str) -> str:
    for line in text.splitlines():
        if line.strip().startswith("|") and needle in line:
            return line
    return ""


def section(text: str, start: str, end: str) -> str:
    opening = re.search(rf"^#{{1,6}}\s+{re.escape(start)}\s*$", text, re.MULTILINE)
    if opening is None:
        return ""
    closing = re.search(
        rf"^#{{1,6}}\s+{re.escape(end)}\s*$", text[opening.end() :], re.MULTILINE
    )
    return text[opening.end() : opening.end() + closing.start()] if closing else ""


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    documents: dict[str, str] = {}
    for relative in DOCUMENTS:
        path = root / relative
        if not path.is_file():
            errors.append(f"missing {relative}")
            continue
        try:
            documents[relative] = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            errors.append(f"{relative}: cannot read: {error}")
    if errors:
        return errors

    inventory = documents[INVENTORY]
    if not re.search(r"^##\s+Retention decisions\b", inventory, re.MULTILINE):
        errors.append("DATA_INVENTORY.md is missing the 'Retention decisions' section")
    decisions = section(
        inventory, "Retention decisions", "D. User-input, biometric, and control classes"
    )
    if DECISION_HEADER not in decisions:
        errors.append("the decision table header changed")
    for dc, (rule, enforced_phrase) in DECISIONS.items():
        cells = decision_cells(decisions, dc)
        if cells is None:
            errors.append(f"no decision row for {dc}")
            continue
        if len(cells) < 5:
            errors.append(f"the {dc} decision row is truncated")
            continue
        if cells[1] != rule:
            errors.append(f"{dc} decision rule is {cells[1]!r}, expected {rule!r}")
        if enforced_phrase not in flatten(cells[2]).replace("`", ""):
            errors.append(f"{dc} decision lost its enforcement statement")
        if dc == "DC-26" and not cells[2].lower().startswith("none"):
            errors.append("DC-26 claims enforcement that does not exist")
        if dc == "DC-21" and "not implemented" not in flatten(cells[3]):
            errors.append("DC-21 planned target lost its not-implemented label")

    dc26 = table_cells(inventory, "DC-26")
    if dc26 is None:
        errors.append("inventory has no DC-26 row")
    elif len(dc26) < 5:
        errors.append("inventory DC-26 row is truncated")
    elif dc26[4] != "7-days":
        errors.append(f"DC-26 retention {dc26[4]!r} is not '7-days'")
    if "stays in DC-26's Retention cell" not in inventory:
        errors.append("the explanation for DC-26's planned 7-days is missing")

    for dc in ("DC-20", "DC-21"):
        row = table_cells(inventory, dc)
        if row is None:
            errors.append(f"inventory has no {dc} row")
        elif len(row) < 5:
            errors.append(f"inventory {dc} row is truncated")
        elif row[4] != "session":
            errors.append(f"{dc} retention {row[4]!r} is not 'session'")

    arch_row = row_containing(documents[ARCHITECTURE], "| Logs |")
    if not arch_row:
        errors.append("ARCHITECTURE.md has no log row")
    else:
        if "7 days" in arch_row:
            errors.append("ARCHITECTURE.md: diagnostic log row still promises 7 days")
        if "7-days planned target, not enforced" not in arch_row:
            errors.append("ARCHITECTURE.md lost the planned-target label")
        if "](DATA_INVENTORY.md)" not in arch_row:
            errors.append("ARCHITECTURE.md log row lost the inventory link")

    threat_row = row_containing(documents[THREAT], "| Action log |")
    if not threat_row:
        errors.append("THREAT_MODEL.md has no action log row")
    else:
        if "30 days" in threat_row:
            errors.append("THREAT_MODEL.md: action log row still promises 30 days")
        if "planned target, not enforced" not in threat_row:
            errors.append("THREAT_MODEL.md lost the planned-target label")
        if "](DATA_INVENTORY.md)" not in threat_row:
            errors.append("THREAT_MODEL.md action log row lost the inventory link")

    confirm = flatten(documents[CONFIRM])
    if "there is no timestamp or expiry" not in confirm:
        errors.append("CONFIRM.md no longer states there is no timestamp or expiry")
    if "](data_inventory.md)" not in confirm:
        errors.append("CONFIRM.md lost the inventory link")
    guard = flatten(documents[WRITE_GUARD])
    if "`bounded-records` (4096)" not in guard:
        errors.append("WRITE_GUARD.md no longer states 'bounded-records (4096)'")
    if "](data_inventory.md)" not in guard:
        errors.append("WRITE_GUARD.md lost the inventory link")
    redaction = flatten(documents[REDACTION])
    if "session-only today" not in redaction:
        errors.append("REDACTION.md no longer states diagnostics are session-only today")
    if "](data_inventory.md)" not in redaction:
        errors.append("REDACTION.md lost the inventory link")

    f04 = table_cells(documents[FINDINGS], "F-04")
    if f04 is None:
        errors.append("PRIVACY_FINDINGS.md has no F-04 row")
    elif len(f04) < 3:
        errors.append("F-04 row is truncated")
    elif f04[2] != "MITIGATED":
        errors.append(f"F-04 status is {f04[2]!r}, not MITIGATED")

    r05 = table_cells(documents[DPIA], "R-05")
    if r05 is None:
        errors.append("DPIA.md has no R-05 row")
    elif len(r05) < 5:
        errors.append("R-05 row is truncated")
    elif "day counts labelled planned targets" not in flatten(r05[4]):
        errors.append("R-05 mitigation does not reflect F-04")
    if "Untested public claims (R-05)" in documents[DPIA]:
        errors.append("DPIA residual risk still lists unreconciled public claims")

    for relative in (CONFIRM_RS, PROVENANCE_RS, WRITE_GUARD_RS, REDACT_RS):
        match = CLOCK_PRIMITIVE.search(documents[relative])
        if match:
            errors.append(f"{relative} invents a time-based rule ({match.group(0)})")
    if "MAX_RETAINED_RECORDS: usize = 4096" not in documents[WRITE_GUARD_RS]:
        errors.append("WRITE_GUARD.rs lost the 4096-record bound")

    readme = flatten(documents[README])
    if "validate_retention.py" not in readme:
        errors.append("README.md no longer indexes scripts/validate_retention.py")
    workflow = documents[WORKFLOW]
    if "scripts/validate_retention.py --self-test" not in workflow:
        errors.append("CI no longer self-tests the retention validator")
    if not re.search(r"python3 scripts/validate_retention\.py\n", workflow):
        errors.append("CI no longer runs the retention validator")
    return errors


def collected_files(root: Path) -> list[str]:
    return list(DOCUMENTS)


def materialize(target: Path, pristine: dict[str, str]) -> None:
    if target.exists():
        shutil.rmtree(target)
    for relative, text in pristine.items():
        path = target / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8", newline="\n")


CASES: tuple[dict[str, object], ...] = (
    {
        "label": "decision section deleted",
        "edit": [(INVENTORY, "## Retention decisions", "## Retention notes")],
        "expect": "missing the 'Retention decisions' section",
    },
    {
        "label": "decision table header changed",
        "edit": [(INVENTORY, DECISION_HEADER, "| Class | Rule | Today | Target | Finding |")],
        "expect": "decision table header changed",
    },
    {
        "label": "DC-20 decision deleted",
        "edit": [(INVENTORY, "| Provenance panel (DC-20) | session |", "| Provenance panel | session |")],
        "expect": "no decision row for DC-20",
    },
    {
        "label": "DC-21 inventory reverts to disputed",
        "edit": [
            (
                INVENTORY,
                "| RAM log today, SQLCipher specified | session | legitimate-interests",
                "| RAM log today, SQLCipher specified | disputed | legitimate-interests",
            )
        ],
        "expect": "DC-21 retention 'disputed'",
    },
    {
        "label": "bare 7 days restored",
        "edit": [
            (
                ARCHITECTURE,
                "| Logs | stderr/caller sink today; disk planned | redacted | 7-days planned target, not enforced; see [DATA_INVENTORY.md](DATA_INVENTORY.md) §Retention decisions |",
                "| Logs | disk | redacted | 7 days, no content |",
            )
        ],
        "expect": "ARCHITECTURE.md: diagnostic log row still promises 7 days",
    },
    {
        "label": "bare 30 days restored",
        "edit": [
            (
                THREAT,
                "| Action log | RAM today; SQLite planned | session (RAM); 30-days is a planned target, not enforced |",
                "| Action log | RAM today; SQLite planned | 30 days |",
            )
        ],
        "expect": "THREAT_MODEL.md: action log row still promises 30 days",
    },
    {
        "label": "confirmation invents expiry",
        "edit": [
            (
                CONFIRM,
                "there is no timestamp or expiry",
                "records expire after 30 days",
            )
        ],
        "expect": "CONFIRM.md no longer states there is no timestamp or expiry",
    },
    {
        "label": "write guard loses the record bound",
        "edit": [(WRITE_GUARD, "`bounded-records` (4096)", "`session`")],
        "expect": "no longer states 'bounded-records (4096)'",
    },
    {
        "label": "redaction loses session-only",
        "edit": [(REDACTION, "session-only today", "retained on disk")],
        "expect": "no longer states diagnostics are session-only today",
    },
    {
        "label": "DC-26 enforcement invented",
        "edit": [
            (
                INVENTORY,
                "none: redacted lines go to stderr or a caller sink; no disk sink and no expiry",
                "enforced by a disk purge",
            )
        ],
        "expect": "DC-26 claims enforcement that does not exist",
    },
    {
        "label": "DC-21 planned target loses its label",
        "edit": [(INVENTORY, "30-days in a SQLCipher store; **not implemented**", "30-days in a SQLCipher store")],
        "expect": "DC-21 planned target lost its not-implemented label",
    },
    {
        "label": "DC-26 inventory cell changed",
        "edit": [
            (
                INVENTORY,
                "| disk, redacted | 7-days | legitimate-interests",
                "| disk, redacted | session | legitimate-interests",
            )
        ],
        "expect": "DC-26 retention 'session' is not '7-days'",
    },
    {
        "label": "DC-26 explanation removed",
        "edit": [
            (
                INVENTORY,
                "`7-days` stays in DC-26's Retention cell because it is the declared rule for",
                "The declared rule for",
            )
        ],
        "expect": "explanation for DC-26's planned 7-days is missing",
    },
    {
        "label": "finding reopened",
        "edit": [(FINDINGS, "| F-04 | MEDIUM | MITIGATED |", "| F-04 | MEDIUM | OPEN |")],
        "expect": "F-04 status",
    },
    {
        "label": "DPIA mitigation stale",
        "edit": [(DPIA, "day counts labelled planned targets", "conflicts flagged")],
        "expect": "R-05 mitigation does not reflect F-04",
    },
    {
        "label": "residual risk stale",
        "edit": [(DPIA, "- **Planned persistence (R-05).**", "- **Untested public claims (R-05).**")],
        "expect": "DPIA residual risk still lists unreconciled public claims",
    },
    {
        "label": "a clock rule appears in the action log",
        "edit": [(CONFIRM_RS, "/// Drops every record.", "/// Drops every record after Duration::from_secs(30).")],
        "expect": "time-based rule",
    },
    {
        "label": "a clock rule appears in the write guard",
        "edit": [
            (
                WRITE_GUARD_RS,
                "pub const MAX_RETAINED_RECORDS: usize = 4096;",
                "pub const MAX_RETAINED_RECORDS: usize = Duration::from_secs(30);",
            )
        ],
        "expect": "time-based rule",
    },
    {
        "label": "a clock rule appears in the diagnostics layer",
        "edit": [(REDACT_RS, "pub struct Logger {", "pub struct Logger { // std::time::Instant")],
        "expect": "time-based rule",
    },
    {
        "label": "readme drops the validator",
        "edit": [(README, "validate_retention.py", "validate_removed.py")],
        "expect": "no longer indexes scripts/validate_retention.py",
    },
    {
        "label": "CI drops the self-test",
        "edit": [
            (WORKFLOW, "scripts/validate_retention.py --self-test", "scripts/validate_hashes.py --self-test")
        ],
        "expect": "no longer self-tests the retention validator",
    },
)


def _apply(tmp: Path, case: dict[str, object]) -> None:
    for relative, old, new in case.get("edit", ()):  # type: ignore[union-attr]
        path = tmp / str(relative)
        text = path.read_text(encoding="utf-8")
        if old == new:
            raise AssertionError(f"case is a no-op: {old!r} in {relative}")
        if old not in text:
            raise AssertionError(f"case no longer applies: {old!r} not in {relative}")
        path.write_text(text.replace(old, new, 1), encoding="utf-8", newline="\n")


def self_test() -> int:
    pristine = {
        relative: (ROOT / relative).read_text(encoding="utf-8")
        for relative in collected_files(ROOT)
    }
    tmp = Path(tempfile.mkdtemp(prefix="curbcut-retention-decisions-"))
    try:
        materialize(tmp, pristine)
        if validate(tmp):
            print("SELF-TEST FAILED: a pristine copy did not pass", file=sys.stderr)
            return 1
        print("self-test: the committed documents pass")

        for case in CASES:
            materialize(tmp, pristine)
            try:
                _apply(tmp, case)
            except AssertionError as error:
                print(
                    f"SELF-TEST FAILED: {case['label']}: {error}", file=sys.stderr
                )
                return 1
            errors = validate(tmp)
            expected = str(case["expect"])
            if not any(expected.lower() in error.lower() for error in errors):
                print(
                    f"SELF-TEST FAILED: {case['label']} was not detected with "
                    f"{expected!r} (got {errors[:2]})",
                    file=sys.stderr,
                )
                return 1

        print(
            f"self-test: {len(CASES)} named attacks were each detected by their "
            "own check"
        )
        return 0
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--self-test",
        action="store_true",
        help=f"apply {len(CASES)} named attacks and require detection",
    )
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    errors = validate(ROOT)
    for message in errors:
        print(f"FAIL: {message}", file=sys.stderr)
    if errors:
        print(f"{len(errors)} retention check(s) failed", file=sys.stderr)
        return 1
    print("each retention class has one enforced rule and labelled planned targets")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
