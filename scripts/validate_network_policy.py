#!/usr/bin/env python3
"""Fail if the update-check exception loses its basis, control, or honesty.

Issue #205 found a published opt-in promise that the code does not keep:
`UpdateCheck` is compiled into every build and denied by a default-engaged
internal kill switch, but no per-user setting exists. `docs/NETWORK_POLICY.md`
now records it as a deliberate exception. This validator checks that the
document, its consumers, the findings register, and the code-level gate stay
consistent with that statement.

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

POLICY = "docs/NETWORK_POLICY.md"
INVENTORY = "docs/DATA_INVENTORY.md"
DPIA = "docs/DPIA.md"
FINDINGS = "docs/PRIVACY_FINDINGS.md"
SECURITY = "SECURITY.md"
THREAT = "docs/THREAT_MODEL.md"
NET_RS = "crates/curbcut-core/src/net.rs"
README = "README.md"
WORKFLOW = ".github/workflows/ci.yml"

DOCUMENTS = (
    POLICY,
    INVENTORY,
    DPIA,
    FINDINGS,
    SECURITY,
    THREAT,
    NET_RS,
    README,
    WORKFLOW,
)

CONSUMERS = (INVENTORY, DPIA, FINDINGS, SECURITY, THREAT)
CONSUMER_PHRASES = {
    INVENTORY: "documented exception in",
    DPIA: "one exception (f-12)",
    FINDINGS: "documented exception",
    SECURITY: "documented compiled-in update check",
    THREAT: "documented exception",
}

SECTIONS = ("The channels", "The modes", "The gate in use", "The update-check exception", "Limits")

REQUIRED_PHRASES = (
    "dc-30",
    "f-12",
    "r-09",
    "#205",
    "#221",
    "deliberate exception",
    "no per-user opt-in",
    "starts engaged",
    "no shipped code path calls release",
    "version only",
    "no screen content",
    "update::dial",
    "broken install",
)

OVERCLAIMS = (
    "user has opted in",
    "user-facing opt-in exists",
    "off by default until the user",
    "fully offline",
    "kill switch is a user control",
    "wcag compliant",
    "ada compliant",
    "makes apps accessible",
)

CODE_INVARIANTS = (
    "pub mod update",
    "if feature != NetworkFeature::UpdateCheck",
    "if !switch.allows(feature)",
    "engaged: true",
    "matches!(self, Self::UpdateCheck)",
)


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

    policy = documents[POLICY]
    for heading in SECTIONS:
        if not re.search(rf"^#{{1,6}}\s+{re.escape(heading)}\b", policy, re.MULTILINE):
            errors.append(f"NETWORK_POLICY.md is missing the {heading!r} section")
    body = flatten(policy)
    for phrase in REQUIRED_PHRASES:
        if phrase not in body:
            errors.append(f"NETWORK_POLICY.md no longer states {phrase!r}")
    for phrase in OVERCLAIMS:
        if phrase in body:
            errors.append(f"NETWORK_POLICY.md overclaims {phrase!r}")

    for relative in CONSUMERS:
        text = documents[relative]
        if "NETWORK_POLICY.md" not in text:
            errors.append(f"{relative} no longer references NETWORK_POLICY.md")
        phrase = CONSUMER_PHRASES[relative]
        if phrase not in flatten(text):
            errors.append(f"{relative} no longer states {phrase!r}")

    f12 = table_cells(documents[FINDINGS], "F-12")
    if f12 is None:
        errors.append("PRIVACY_FINDINGS.md has no F-12 row")
    elif len(f12) < 3:
        errors.append("F-12 row is truncated")
    elif f12[2] != "MITIGATED":
        errors.append(f"F-12 status is {f12[2]!r}, not MITIGATED")

    r09 = table_cells(documents[DPIA], "R-09")
    if r09 is None:
        errors.append("DPIA.md has no R-09 row")
    elif len(r09) < 5:
        errors.append("R-09 row is truncated")
    elif "documented exception" not in flatten(r09[4]):
        errors.append("R-09 mitigation does not name the documented exception")
    elif "F-12" not in " ".join(r09):
        errors.append("R-09 no longer points at F-12")

    net = documents[NET_RS]
    for phrase in CODE_INVARIANTS:
        if phrase not in net:
            errors.append(f"net.rs no longer contains {phrase!r}")

    readme = flatten(documents[README])
    if "network_policy.md" not in readme:
        errors.append("README.md no longer indexes docs/NETWORK_POLICY.md")
    if "validate_network_policy.py" not in readme:
        errors.append("README.md no longer indexes scripts/validate_network_policy.py")
    workflow = documents[WORKFLOW]
    if "scripts/validate_network_policy.py --self-test" not in workflow:
        errors.append("CI no longer self-tests the network-policy validator")
    if not re.search(r"python3 scripts/validate_network_policy\.py\n", workflow):
        errors.append("CI no longer runs the network-policy validator")
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
        "delete": [POLICY],
        "expect": "missing docs/NETWORK_POLICY.md",
    },
    {
        "label": "exception section renamed",
        "edit": [(POLICY, "## The update-check exception", "## Exception")],
        "expect": "'The update-check exception' section",
    },
    {
        "label": "opt-in invented",
        "edit": [(POLICY, "no per-user opt-in", "a user-facing opt-in")],
        "expect": "'no per-user opt-in'",
    },
    {
        "label": "user consent overclaim",
        "edit": [(POLICY, "There is **no per-user opt-in**", "The user has opted in to updates")],
        "expect": "overclaims 'user has opted in'",
    },
    {
        "label": "switch default inverted",
        "replace": [(POLICY, "starts engaged", "starts disengaged")],
        "expect": "'starts engaged'",
    },
    {
        "label": "shipped release caller invented",
        "edit": [(POLICY, "No shipped code path calls\nrelease()", "A shipped code path calls\nrelease()")],
        "expect": "no shipped code path calls release",
    },
    {
        "label": "issue reference changed",
        "edit": [(POLICY, "#205", "#999")],
        "expect": "'#205'",
    },
    {
        "label": "finding closed early",
        "edit": [(FINDINGS, "| F-12 | MEDIUM | MITIGATED |", "| F-12 | MEDIUM | OPEN |")],
        "expect": "F-12 status",
    },
    {
        "label": "R-09 loses the exception",
        "edit": [(DPIA, "documented exception in [`docs/NETWORK_POLICY.md`](NETWORK_POLICY.md); no per-user opt-in (#221)", "no user choice")],
        "expect": "R-09 mitigation does not name the documented exception",
    },
    {
        "label": "update dial loses the switch check",
        "replace": [(NET_RS, "if !switch.allows(feature)", "if false")],
        "expect": "net.rs no longer contains 'if !switch.allows(feature)'",
    },
    {
        "label": "readme drops the validator",
        "edit": [(README, "validate_network_policy.py", "validate_removed.py")],
        "expect": "no longer indexes scripts/validate_network_policy.py",
    },
    {
        "label": "CI drops the self-test",
        "edit": [(WORKFLOW, "scripts/validate_network_policy.py --self-test", "scripts/validate_hashes.py --self-test")],
        "expect": "no longer self-tests the network-policy validator",
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

    tmp = Path(tempfile.mkdtemp(prefix="curbcut-network-policy-"))
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
        print(f"{len(errors)} network-policy check(s) failed", file=sys.stderr)
        return 1
    print("the update-check exception is documented with its basis and real control")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
