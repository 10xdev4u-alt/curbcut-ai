#!/usr/bin/env python3
"""Fail if an off-device path is described as implemented or as a channel.

Issue #203 found two proposed transmission paths — crash reports (DC-27) and
mobile companion remote read-aloud (DC-29) — with no documented channel,
approval step, or revocation. `docs/TRANSMISSION_PATHS.md` now records both as
planned and states plainly that no code exists. This validator checks that the
document, its consumers, and the code-level channel list stay honest.

Run ``--self-test`` to apply every named mutation to a pristine copy and
require detection. Every mutation source is preflighted, so a stale fixture
fails cleanly instead of silently passing.
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

PATHS = "docs/TRANSMISSION_PATHS.md"
INVENTORY = "docs/DATA_INVENTORY.md"
DPIA = "docs/DPIA.md"
FINDINGS = "docs/PRIVACY_FINDINGS.md"
INCIDENT = "docs/INCIDENT_RESPONSE.md"
NETWORK_POLICY = "docs/NETWORK_POLICY.md"
THREAT = "docs/THREAT_MODEL.md"
NET_RS = "crates/curbcut-core/src/net.rs"
README = "README.md"
WORKFLOW = ".github/workflows/ci.yml"

DOCUMENTS = (
    PATHS,
    INVENTORY,
    DPIA,
    FINDINGS,
    INCIDENT,
    NETWORK_POLICY,
    THREAT,
    NET_RS,
    README,
    WORKFLOW,
)

CONSUMERS = (INVENTORY, DPIA, FINDINGS, INCIDENT, NETWORK_POLICY, THREAT)

# The inventory's retention section legitimately says "Enforced today", so
# that phrase is checked only in the canonical document. These claims are
# always false for the two off-device paths.
CONSUMER_OVERCLAIMS = (
    "is implemented",
    "user can revoke today",
    "wcag compliant",
    "ada compliant",
    "makes apps accessible",
    "channels exist",
    "crash reports are sent",
    "text is sent to a phone",
)

CONSUMER_PHRASES = {
    INVENTORY: "both are planned and not implemented today",
    DPIA: "channel undocumented",
    FINDINGS: "no channel or revocation control exists",
    INCIDENT: "neither crash-report upload nor companion read-aloud is a channel today",
    NETWORK_POLICY: "crash reports and companion read-aloud are not channels today",
    THREAT: "not an implemented channel",
}

SECTIONS = (
    "Status",
    "Crash reports",
    "Mobile companion",
    "What is sent, when, through which channel",
    "Gate and revocation",
    "Never stored or logged",
    "Open work and tracking",
)

REQUIRED_PHRASES = (
    "dc-27",
    "dc-29",
    "f-08",
    "r-07",
    "#203",
    "#219",
    "planned",
    "no code",
    "no network call",
    "off by default",
    "opt-in",
    "revocation",
    "counts and hashes only",
    "not implemented",
    "networkfeature",
    "killswitch",
    "no approval ui",
)

OVERCLAIMS = (
    "wcag compliant",
    "ada compliant",
    "makes apps accessible",
    "enforced today",
    "is implemented",
    "user can revoke today",
)

CHANNEL_LIST = "pub const ALL: [NetworkFeature; 5] = ["


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

    text = documents[PATHS]
    for heading in SECTIONS:
        if not re.search(rf"^#{{1,6}}\s+{re.escape(heading)}\b", text, re.MULTILINE):
            errors.append(f"TRANSMISSION_PATHS.md is missing the {heading!r} section")
    body = flatten(text)
    for phrase in REQUIRED_PHRASES:
        if phrase not in body:
            errors.append(f"TRANSMISSION_PATHS.md no longer states {phrase!r}")
    for phrase in OVERCLAIMS:
        if phrase in body:
            errors.append(f"TRANSMISSION_PATHS.md overclaims {phrase!r}")

    for relative in CONSUMERS:
        text = documents[relative]
        if "](TRANSMISSION_PATHS.md)" not in text:
            errors.append(f"{relative} no longer links to docs/TRANSMISSION_PATHS.md")
        lowered = flatten(text)
        required = CONSUMER_PHRASES[relative]
        if required not in lowered:
            errors.append(f"{relative} no longer states {required!r}")
        for phrase in CONSUMER_OVERCLAIMS:
            if phrase in lowered:
                errors.append(f"{relative} overclaims {phrase!r}")

    for dc in ("DC-27", "DC-29"):
        row = table_cells(documents[INVENTORY], dc)
        if row is None:
            errors.append(f"DATA_INVENTORY.md has no {dc} row")
        elif len(row) < 10:
            errors.append(f"{dc} row is truncated")
        elif row[9] != "planned":
            errors.append(f"{dc} code status is {row[9]!r}, not 'planned'")

    f08 = table_cells(documents[FINDINGS], "F-08")
    if f08 is None:
        errors.append("PRIVACY_FINDINGS.md has no F-08 row")
    elif len(f08) < 3:
        errors.append("F-08 row is truncated")
    elif f08[2] != "OPEN":
        errors.append(f"F-08 status is {f08[2]!r}, not OPEN")
    elif f08[-1].strip() != "#203":
        errors.append("F-08 no longer tracks #203")

    r07 = table_cells(documents[DPIA], "R-07")
    if r07 is None:
        errors.append("DPIA.md has no R-07 row")
    elif len(r07) < 5:
        errors.append("R-07 row is truncated")
    elif "channel undocumented" not in flatten(r07[4]):
        errors.append("R-07 no longer states the channel is undocumented")
    if "real until #203 documents them" not in flatten(documents[DPIA]):
        errors.append("DPIA residual risk no longer tracks #203")

    if CHANNEL_LIST not in documents[NET_RS]:
        errors.append("net.rs invents a channel beyond the five existing features")

    readme = flatten(documents[README])
    if "transmission_paths.md" not in readme:
        errors.append("README.md no longer indexes docs/TRANSMISSION_PATHS.md")
    if "validate_transmission_paths.py" not in readme:
        errors.append("README.md no longer indexes scripts/validate_transmission_paths.py")
    workflow = documents[WORKFLOW]
    if "scripts/validate_transmission_paths.py --self-test" not in workflow:
        errors.append("CI no longer self-tests the transmission-path validator")
    if not re.search(r"python3 scripts/validate_transmission_paths\.py\n", workflow):
        errors.append("CI no longer runs the transmission-path validator")
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
        "label": "canonical document deleted",
        "delete": [PATHS],
        "expect": "missing docs/TRANSMISSION_PATHS.md",
    },
    {
        "label": "status section renamed",
        "edit": [(PATHS, "## Status", "## Current state")],
        "expect": "'Status' section",
    },
    {
        "label": "paths marked implemented",
        "edit": [(PATHS, "Both paths are **planned — not implemented**.", "Both paths are is implemented.")],
        "expect": "overclaims 'is implemented'",
    },
    {
        "label": "companion no longer off by default",
        "replace": [(PATHS, "off by default", "on by default")],
        "expect": "'off by default'",
    },
    {
        "label": "a network call is invented",
        "edit": [(PATHS, "No network call exists for either path today.", "Crash reports upload over HTTPS.")],
        "expect": "'no network call'",
    },
    {
        "label": "an approval UI is invented",
        "replace": [
            (PATHS, "no approval UI", "an approval UI"),
            (PATHS, "No approval UI", "An approval UI"),
        ],
        "expect": "'no approval ui'",
    },
    {
        "label": "content logging slip",
        "replace": [(PATHS, "and hashes only", "or raw text")],
        "expect": "'counts and hashes only'",
    },
    {
        "label": "DC-27 marked implemented",
        "edit": [
            (
                INVENTORY,
                "| explicit approval before anything is sent | opt-in | hash-only | planned |",
                "| explicit approval before anything is sent | opt-in | hash-only | implemented |",
            )
        ],
        "expect": "DC-27 code status",
    },
    {
        "label": "DC-29 marked implemented",
        "edit": [
            (
                INVENTORY,
                "| off by default, user-held key | opt-in | derived | planned |",
                "| off by default, user-held key | opt-in | derived | implemented |",
            )
        ],
        "expect": "DC-29 code status",
    },
    {
        "label": "finding closed early",
        "edit": [(FINDINGS, "| F-08 | HIGH | OPEN |", "| F-08 | HIGH | MITIGATED |")],
        "expect": "F-08 status",
    },
    {
        "label": "residual risk drops the tracker",
        "edit": [(DPIA, "Real until #203 documents them", "Real until the paths are written")],
        "expect": "DPIA residual risk no longer tracks #203",
    },
    {
        "label": "DPIA drops the link",
        "replace": [(DPIA, "[`docs/TRANSMISSION_PATHS.md`](TRANSMISSION_PATHS.md)", "the transmission doc")],
        "expect": "DPIA.md no longer links",
    },
    {
        "label": "incident plan drops the link",
        "edit": [(INCIDENT, "[`TRANSMISSION_PATHS.md`](TRANSMISSION_PATHS.md)", "the transmission doc")],
        "expect": "INCIDENT_RESPONSE.md no longer links",
    },
    {
        "label": "incident plan forgets there is no channel",
        "edit": [
            (
                INCIDENT,
                "Neither crash-report upload nor companion read-aloud is a channel today",
                "Both crash-report upload and companion read-aloud are live channels",
            )
        ],
        "expect": "INCIDENT_RESPONSE.md no longer states",
    },
    {
        "label": "network policy forgets there is no channel",
        "edit": [
            (
                NETWORK_POLICY,
                "Crash reports and companion read-aloud are not channels today",
                "Crash reports and companion read-aloud are live channels today",
            )
        ],
        "expect": "NETWORK_POLICY.md no longer states",
    },
    {
        "label": "threat model invents an implemented channel",
        "edit": [(THREAT, "not an implemented channel", "an implemented channel")],
        "expect": "THREAT_MODEL.md no longer states",
    },
    {
        "label": "incident plan claims channels exist",
        "edit": [(INCIDENT, "process failure, not a success.", "process failure, not a success. Crash channels exist.")],
        "expect": "INCIDENT_RESPONSE.md overclaims 'channels exist'",
    },
    {
        "label": "a sixth network channel is invented",
        "edit": [(NET_RS, CHANNEL_LIST, "pub const ALL: [NetworkFeature; 6] = [")],
        "expect": "invents a channel",
    },
    {
        "label": "readme drops the validator",
        "edit": [(README, "validate_transmission_paths.py", "validate_removed.py")],
        "expect": "no longer indexes scripts/validate_transmission_paths.py",
    },
    {
        "label": "CI drops the self-test",
        "edit": [
            (WORKFLOW, "scripts/validate_transmission_paths.py --self-test", "scripts/validate_hashes.py --self-test")
        ],
        "expect": "no longer self-tests the transmission-path validator",
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
        for relative in case.get("delete", ()):  # type: ignore[union-attr]
            if str(relative) not in pristine:
                print(
                    f"SELF-TEST FAILED: stale deletion in {case['label']}: {relative}",
                    file=sys.stderr,
                )
                return 1

    tmp = Path(tempfile.mkdtemp(prefix="curbcut-transmission-"))
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
        print(f"{len(errors)} transmission-path check(s) failed", file=sys.stderr)
        return 1
    print("the transmission paths are documented without implying an implemented channel")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
