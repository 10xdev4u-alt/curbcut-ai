#!/usr/bin/env python3
"""Fail if telemetry retention drifts back to an unenforceable 90-day claim.

Issue #199 found a published retention rule with no mechanism: the telemetry
crate keeps counters in RAM, writes nothing to disk, and makes no network call,
yet two summary tables promised 90 days. This script re-reads the canonical
inventory and its summaries and checks the facts that matter:

* ``docs/DATA_INVENTORY.md`` records DC-25 as ``never-stored``.
* The telemetry rows in ``docs/ARCHITECTURE.md`` and ``docs/THREAT_MODEL.md``
  agree with it and no longer publish a bare "90 days".
* ``docs/TELEMETRY.md`` states the RAM-only rule, the absence of disk/network
  primitives, and links the canonical inventory.
* ``docs/PRIVACY_FINDINGS.md`` keeps F-03 mitigated.
* No file in ``crates/curbcut-telemetry/src`` uses a disk or network primitive.
* ``README.md`` indexes this script and CI runs it.

This checks prose and a boundary-level code invariant; the schema itself is
covered by ``cargo test``. Run ``--self-test`` to apply every named mutation to
a pristine copy and require detection.
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
TELEMETRY_DOC = "docs/TELEMETRY.md"
FINDINGS = "docs/PRIVACY_FINDINGS.md"
SOURCE_DIR = "crates/curbcut-telemetry/src"
SOURCES = tuple(
    path.relative_to(ROOT).as_posix()
    for path in sorted((ROOT / SOURCE_DIR).rglob("*.rs"))
)
README = "README.md"
WORKFLOW = ".github/workflows/ci.yml"

DOCUMENTS = (
    INVENTORY,
    ARCHITECTURE,
    THREAT,
    TELEMETRY_DOC,
    FINDINGS,
    README,
    WORKFLOW,
    *SOURCES,
)

CODE_PRIMITIVES = re.compile(r"std::fs|File::open|TcpStream|UdpSocket|std::net")


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


def row_containing(text: str, needle: str) -> str:
    for line in text.splitlines():
        if line.strip().startswith("|") and needle in line:
            return line
    return ""


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

    # The inventory is the single source of truth (docs/DATA_INVENTORY.md:4).
    dc25 = table_cells(documents[INVENTORY], "DC-25")
    if dc25 is None:
        errors.append("DATA_INVENTORY.md has no DC-25 row")
    else:
        if len(dc25) < 5:
            errors.append("DC-25 row is truncated")
        elif dc25[4] != "never-stored":
            errors.append(
                f"DC-25 retention is {dc25[4]!r}, not 'never-stored'"
            )
        if len(dc25) > 10 and dc25[9] != "in-memory":
            errors.append(f"DC-25 code status is {dc25[9]!r}, not 'in-memory'")
        if "F-03" not in " ".join(dc25):
            errors.append("DC-25 no longer points at F-03")

    for relative, label in (
        (ARCHITECTURE, "ARCHITECTURE.md"),
        (THREAT, "THREAT_MODEL.md"),
    ):
        row = row_containing(documents[relative], "| Telemetry |")
        if not row:
            errors.append(f"{label} has no telemetry row")
            continue
        if "90 days" in row or "90-days" in row:
            errors.append(f"{label}: telemetry row still promises 90 days")
        if "never-stored" not in row:
            errors.append(f"{label}: telemetry row lost the never-stored rule")

    telemetry = flatten(documents[TELEMETRY_DOC])
    for phrase in (
        "never-stored",
        "there is no disk write, no network call, and no time-based expiry",
        "nothing is written to disk, and no network call exists in this crate",
        "](data_inventory.md)",
    ):
        if phrase not in telemetry:
            errors.append(f"TELEMETRY.md no longer states {phrase!r}")

    f03 = table_cells(documents[FINDINGS], "F-03")
    if f03 is None:
        errors.append("PRIVACY_FINDINGS.md has no F-03 row")
    elif len(f03) < 3:
        errors.append("F-03 row is truncated")
    elif f03[2] in {"OPEN", "IN PROGRESS"}:
        errors.append("F-03 is OPEN; the telemetry claim is still unreconciled")

    for relative in SOURCES:
        match = CODE_PRIMITIVES.search(documents[relative])
        if match:
            errors.append(
                f"{relative} uses a disk or network primitive ({match.group(0)})"
            )

    readme = flatten(documents[README])
    if "validate_telemetry_retention.py" not in readme:
        errors.append("README.md no longer indexes scripts/validate_telemetry_retention.py")

    workflow = documents[WORKFLOW]
    if "scripts/validate_telemetry_retention.py --self-test" not in workflow:
        errors.append("CI no longer self-tests the telemetry-retention validator")
    if not re.search(r"python3 scripts/validate_telemetry_retention\.py\n", workflow):
        errors.append("CI no longer runs the telemetry-retention validator")
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
        "label": "inventory DC-25 reverts to 90 days",
        "edit": [
            (
                INVENTORY,
                "| RAM only | never-stored | consent | opt-in export",
                "| RAM only | 90-days | consent | opt-in export",
            )
        ],
        "expect": "DC-25 retention",
    },
    {
        "label": "architecture summary reverts to 90 days",
        "edit": [
            (
                ARCHITECTURE,
                "| Telemetry | RAM only | n/a | never-stored (opt-in export) |",
                "| Telemetry | local only | n/a | opt-in, 90 days |",
            )
        ],
        "expect": "ARCHITECTURE.md: telemetry row still promises 90 days",
    },
    {
        "label": "threat model reverts to 90 days",
        "edit": [
            (
                THREAT,
                "| Telemetry | RAM only | never-stored | n/a |",
                "| Telemetry | local | 90 days | n/a |",
            )
        ],
        "expect": "THREAT_MODEL.md: telemetry row still promises 90 days",
    },
    {
        "label": "retention section loses never-stored",
        "edit": [(TELEMETRY_DOC, "The canonical rule for DC-25 is `never-stored`:", "The rule is:")],
        "expect": "'never-stored'",
    },
    {
        "label": "retention section loses the expiry denial",
        "edit": [
            (
                TELEMETRY_DOC,
                "there is no disk write, no network call, and no time-based expiry",
                "counts are kept for a window",
            )
        ],
        "expect": "no disk write, no network call, and no time-based expiry",
    },
    {
        "label": "exporter section loses the disk/network denial",
        "edit": [
            (
                TELEMETRY_DOC,
                "Nothing is written to disk, and no network call exists in this crate.",
                "Exports may be sent to a collector.",
            )
        ],
        "expect": "nothing is written to disk, and no network call exists in this crate",
    },
    {
        "label": "telemetry doc drops the inventory link",
        "edit": [(TELEMETRY_DOC, "[`docs/DATA_INVENTORY.md`](DATA_INVENTORY.md)", "the inventory")],
        "expect": "'](data_inventory.md)'",
    },
    {
        "label": "finding reopened",
        "edit": [(FINDINGS, "| F-03 | MEDIUM | MITIGATED |", "| F-03 | MEDIUM | OPEN |")],
        "expect": "F-03 is OPEN",
    },
    {
        "label": "a disk primitive appears in the telemetry crate",
        "edit": [
            (
                "crates/curbcut-telemetry/src/exporter.rs",
                "//! The opt-in telemetry exporter.",
                "//! The opt-in telemetry exporter. std::fs::write",
            )
        ],
        "expect": "disk or network primitive",
    },
    {
        "label": "readme drops the validator",
        "edit": [(README, "validate_telemetry_retention.py", "validate_removed.py")],
        "expect": "no longer indexes scripts/validate_telemetry_retention.py",
    },
    {
        "label": "CI drops the self-test",
        "edit": [
            (
                WORKFLOW,
                "scripts/validate_telemetry_retention.py --self-test",
                "scripts/validate_data_inventory.py --self-test",
            )
        ],
        "expect": "no longer self-tests the telemetry-retention validator",
    },
)


def _apply(tmp: Path, case: dict[str, object]) -> None:
    for relative in case.get("delete", ()):  # type: ignore[union-attr]
        (tmp / str(relative)).unlink(missing_ok=True)
    for relative, old, new in case.get("edit", ()):  # type: ignore[union-attr]
        path = tmp / str(relative)
        text = path.read_text(encoding="utf-8")
        if old not in text:
            raise AssertionError(f"case no longer applies: {old!r} not in {relative}")
        path.write_text(text.replace(old, new, 1), encoding="utf-8", newline="\n")


def self_test() -> int:
    pristine = {
        relative: (ROOT / relative).read_text(encoding="utf-8")
        for relative in collected_files(ROOT)
    }
    tmp = Path(tempfile.mkdtemp(prefix="curbcut-retention-"))
    try:
        materialize(tmp, pristine)
        if validate(tmp):
            print("SELF-TEST FAILED: a pristine copy did not pass", file=sys.stderr)
            return 1
        print("self-test: the committed documents pass")

        for case in CASES:
            materialize(tmp, pristine)
            _apply(tmp, case)
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
        print(f"{len(errors)} telemetry-retention check(s) failed", file=sys.stderr)
        return 1
    print("telemetry retention is never-stored in the inventory and both summaries")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
