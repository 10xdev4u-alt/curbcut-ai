#!/usr/bin/env python3
"""Fail if the DPIA loses its canonical "not audited" statement.

Issue #204 cannot be closed in a repository: commissioning an external privacy
audit needs a named reviewer, a scope, and a signed report from outside this
project. `docs/DPIA.md` §Audit status now states that the DPIA is internal and
unaudited, what a future audit must cover, and what evidence would close F-11.
This validator checks that the statement, its cross-references, and the open
finding stay intact. It cannot prove that an audit happened; the explicit
document set is the boundary of its claim.

Run ``--self-test`` to apply every named mutation to a pristine copy and
require detection. Every mutation source is preflighted.
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

DPIA = "docs/DPIA.md"
FINDINGS = "docs/PRIVACY_FINDINGS.md"
INVENTORY = "docs/DATA_INVENTORY.md"
README = "README.md"
WORKFLOW = ".github/workflows/ci.yml"

DOCUMENTS = (DPIA, FINDINGS, INVENTORY, README, WORKFLOW)

REQUIRED_PHRASES = (
    "internal engineering document",
    "not audited",
    "no external party",
    "all 37 data classes",
    "lawful bases",
    "retention",
    "consent",
    "signed report",
    "named reviewer",
    "security lead",
    "legal counsel",
    "must commission",
    "f-11",
    "#204",
    "does not assert compliance",
    "not legal advice",
)

OVERCLAIMS = (
    "has been audited",
    "was audited",
    "audit completed",
    "audit passed",
    "externally audited",
    "external review completed",
    "asserts compliance",
    "is compliant",
    "compliant with gdpr",
)


def flatten(text: str) -> str:
    return " ".join(text.lower().replace("`", "").replace("*", "").split())


def table_cells(text: str, first_cell: str) -> list[str] | None:
    for line in text.splitlines():
        stripped = line.strip()
        if not (stripped.startswith("|") and stripped.endswith("|")):
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if cells and cells[0] == first_cell:
            return cells
    return None


def section(text: str, start: str, end: str) -> str:
    opening = re.search(rf"^##\s+{re.escape(start)}\s*$", text, re.MULTILINE)
    if opening is None:
        return ""
    closing = re.search(rf"^##\s+{re.escape(end)}\s*$", text[opening.end() :], re.MULTILINE)
    return (
        text[opening.end() : opening.end() + closing.start()]
        if closing
        else text[opening.end() :]
    )


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

    dpia = documents[DPIA]
    if not re.search(r"^##\s+Audit status\s*$", dpia, re.MULTILINE):
        errors.append("DPIA.md is missing the 'Audit status' section")
    block = section(dpia, "Audit status", "What would change this assessment")
    body = flatten(block)
    for phrase in REQUIRED_PHRASES:
        if phrase not in body:
            errors.append(f"DPIA.md §Audit status no longer states {phrase!r}")
    for phrase in OVERCLAIMS:
        if phrase in body:
            errors.append(f"DPIA.md §Audit status overclaims {phrase!r}")
    for relative in (DPIA, FINDINGS, README):
        for phrase in OVERCLAIMS:
            if phrase in flatten(documents[relative]):
                errors.append(f"{relative} overclaims {phrase!r}")

    for heading in ("Status", "Consultation and review"):
        part = section(dpia, heading, "Scope" if heading == "Status" else "Audit status")
        if "§Audit status" not in part:
            errors.append(f"DPIA.md §{heading} no longer points to Audit status")

    findings = documents[FINDINGS]
    f11 = table_cells(findings, "F-11")
    if f11 is None:
        errors.append("PRIVACY_FINDINGS.md has no F-11 row")
    elif len(f11) < 3:
        errors.append("F-11 row is truncated")
    else:
        if f11[1] != "HIGH":
            errors.append(f"F-11 severity is {f11[1]!r}, not HIGH")
        if f11[2] != "OPEN":
            errors.append(f"F-11 status is {f11[2]!r}, not OPEN")
        if f11[-1].strip() != "#204":
            errors.append("F-11 no longer tracks #204")
        summary = flatten(" ".join(f11))
        if "dpi a.md" not in summary and "dpia.md" not in summary:
            errors.append("F-11 no longer links to DPIA.md")
        if "audit status" not in summary:
            errors.append("F-11 no longer references §Audit status")

    inventory = section(
        documents[INVENTORY],
        "What this inventory does not cover",
        "END_OF_DOCUMENT_SENTINEL",
    )
    if "Audit status" not in inventory:
        errors.append("DATA_INVENTORY.md no longer points to §Audit status")

    readme = flatten(documents[README])
    if "not audited" not in readme:
        errors.append("README.md no longer says the DPIA is not audited")
    if "validate_privacy_audit.py" not in readme:
        errors.append("README.md no longer indexes scripts/validate_privacy_audit.py")

    workflow = documents[WORKFLOW]
    if "scripts/validate_privacy_audit.py --self-test" not in workflow:
        errors.append("CI no longer self-tests the privacy-audit status validator")
    if not re.search(r"python3 scripts/validate_privacy_audit\.py\n", workflow):
        errors.append("CI no longer runs the privacy-audit status validator")
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
        "label": "audit-status heading renamed",
        "edit": [(DPIA, "## Audit status", "## Audit state")],
        "expect": "missing the 'Audit status' section",
    },
    {
        "label": "internal status replaced",
        "edit": [(DPIA, "internal engineering document, not audited", "externally reviewed document, audited")],
        "expect": "'not audited'",
    },
    {
        "label": "an audit is claimed",
        "edit": [(DPIA, "do not describe this DPIA as audited", "this DPIA has been audited")],
        "expect": "overclaims 'has been audited'",
    },
    {
        "label": "external reviewer claimed",
        "edit": [(DPIA, "**No external\nparty** has reviewed it", "An external\nparty has reviewed it")],
        "expect": "'no external party'",
    },
    {
        "label": "audit scope shrunk",
        "edit": [(DPIA, "**all 37 data classes**", "a sample of data classes")],
        "expect": "'all 37 data classes'",
    },
    {
        "label": "signed report removed",
        "edit": [(DPIA, "**signed report**", "verbal summary")],
        "expect": "'signed report'",
    },
    {
        "label": "commissioning authority removed",
        "edit": [(DPIA, "must **commission** an audit", "needs no review at all")],
        "expect": "'must commission'",
    },
    {
        "label": "trackers changed",
        "replace": [(DPIA, "F-11", "F-99"), (DPIA, "#204", "#999")],
        "expect": "no longer states",
    },
    {
        "label": "compliance disclaimed as claimed",
        "edit": [(DPIA, "**does not assert compliance**", "asserts compliance")],
        "expect": "'does not assert compliance'",
    },
    {
        "label": "finding closed early",
        "edit": [(FINDINGS, "| F-11 | HIGH | OPEN |", "| F-11 | HIGH | MITIGATED |")],
        "expect": "F-11 status",
    },
    {
        "label": "finding claims an audit happened",
        "edit": [
            (
                FINDINGS,
                "§Audit status records the scope",
                "§Audit status records the scope; this DPIA has been audited",
            )
        ],
        "expect": "PRIVACY_FINDINGS.md overclaims 'has been audited'",
    },
    {
        "label": "finding drops the cross-reference",
        "edit": [(FINDINGS, "§Audit status records the scope", "the DPIA records the scope")],
        "expect": "F-11 no longer references",
    },
    {
        "label": "inventory drops the cross-reference",
        "edit": [(INVENTORY, "§Audit status.", "the DPIA.")],
        "expect": "DATA_INVENTORY.md no longer points",
    },
    {
        "label": "readme drops the validator",
        "edit": [(README, "validate_privacy_audit.py", "validate_removed.py")],
        "expect": "no longer indexes scripts/validate_privacy_audit.py",
    },
    {
        "label": "CI drops the self-test",
        "edit": [(WORKFLOW, "scripts/validate_privacy_audit.py --self-test", "scripts/validate_hashes.py --self-test")],
        "expect": "no longer self-tests the privacy-audit status validator",
    },
)


def _apply(tmp: Path, case: dict[str, object]) -> None:
    for relative in case.get("delete", ()):  # type: ignore[union-attr]
        (tmp / str(relative)).unlink(missing_ok=True)
    for relative, old, new in case.get("edit", ()):  # type: ignore[union-attr]
        path = tmp / str(relative)
        text = path.read_text(encoding="utf-8")
        if old == new:
            raise AssertionError(f"case is a no-op: {old!r} in {relative}")
        if old not in text:
            raise AssertionError(f"case no longer applies: {old!r} not in {relative}")
        path.write_text(text.replace(old, new, 1), encoding="utf-8", newline="\n")
    for relative, old, new in case.get("replace", ()):  # type: ignore[union-attr]
        path = tmp / str(relative)
        text = path.read_text(encoding="utf-8")
        if old == new:
            raise AssertionError(f"case is a no-op: {old!r} in {relative}")
        if old not in text:
            raise AssertionError(f"case no longer applies: {old!r} not in {relative}")
        path.write_text(text.replace(old, new), encoding="utf-8", newline="\n")


def self_test() -> int:
    pristine = {
        relative: (ROOT / relative).read_text(encoding="utf-8")
        for relative in collected_files(ROOT)
    }
    for case in CASES:
        for action in ("edit", "replace"):
            for relative, old, _ in case.get(action, ()):  # type: ignore[union-attr]
                if old not in pristine[str(relative)]:
                    print(
                        f"SELF-TEST FAILED: stale fixture in {case['label']}: "
                        f"{old!r} not in {relative}",
                        file=sys.stderr,
                    )
                    return 1

    tmp = Path(tempfile.mkdtemp(prefix="curbcut-privacy-audit-"))
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
                print(f"SELF-TEST FAILED: {case['label']}: {error}", file=sys.stderr)
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
        print(f"{len(errors)} privacy-audit status check(s) failed", file=sys.stderr)
        return 1
    print("one canonical audit-status statement, four cross-references, and an open F-11")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
