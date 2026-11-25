#!/usr/bin/env python3
"""Fail if the recorded two-build measurement is edited into a false claim.

Issue #208 asks for two independent builds compared by hash. Two release builds
of one revision were recorded in isolated worktrees on **one machine** and their
digests matched. `supply-chain/reproducibility.json` holds the raw record and
`docs/SUPPLY_CHAIN.md` §Two-build measurement explains the limit: this is path
independence, not the required two-environment comparison. This validator
checks that the record and the wording stay consistent, and that the honest
"not proven" statement survives.

It proves nothing about reproducibility itself: the recorded digests are data,
not a new build. Run ``--self-test`` for named mutations.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

RECORD = "supply-chain/reproducibility.json"
DOC = "docs/SUPPLY_CHAIN.md"
README = "README.md"
WORKFLOW = ".github/workflows/ci.yml"

DOCUMENTS = (RECORD, DOC, README, WORKFLOW)

SECTION = "## Two-build measurement (one machine)"
NOT_PROVEN = "Two independent builds have not been shown to match by hash"
REVISION = "2c771b94dc363c028b9846bf785e04d7323bd431"
DIGEST = "d7b9c351b03a1cc3b3c33c8bb526820ce99f68301ea7c397c33db59341bbeec1"
BYTES = 376816

DOC_PHRASES = (
    NOT_PROVEN,
    REVISION,
    DIGEST,
    "376816",
    "cargo build --release --locked -p curbcut-cli",
    "rustc 1.99.1",
    "x86_64-unknown-linux-gnu",
    "supply-chain/reproducibility.json",
    "#208 stays OPEN",
    "not** the two-independent-environments comparison",
)


def flatten(text: str) -> str:
    return " ".join(text.lower().replace("`", "").replace("*", "").split())


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

    try:
        record = json.loads(documents[RECORD])
    except json.JSONDecodeError as error:
        return [f"{RECORD}: invalid JSON: {error}"]
    if not isinstance(record, dict):
        return [f"{RECORD}: expected a JSON object"]

    if record.get("schema") != "curbcut-reproducibility-v1":
        errors.append("the reproducibility record has an unknown schema")
    if record.get("revision") != REVISION:
        errors.append("the recorded revision is not the measured revision")
    if record.get("tracking") != "#208":
        errors.append("the record no longer tracks #208")
    if record.get("digests_match") is not True:
        errors.append("the record does not say the digests match")
    if record.get("independent_environments") is not False:
        errors.append("the record claims independent environments that did not exist")

    builds = record.get("builds")
    if not isinstance(builds, list) or len(builds) != 2:
        errors.append("the record needs exactly two builds")
    else:
        if [build.get("label") for build in builds] != ["A", "B"]:
            errors.append("the builds are not labelled A and B")
        for build in builds:
            if build.get("bytes") != BYTES:
                errors.append(f"build {build.get('label')} has an unrecorded byte count")
            digest = build.get("sha256")
            if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
                errors.append(f"build {build.get('label')} has an invalid digest")
            elif digest != DIGEST:
                errors.append(f"build {build.get('label')} has a different digest")
        if builds[0].get("sha256") != builds[1].get("sha256"):
            errors.append("the recorded digests do not match")

    doc = documents[DOC]
    if SECTION not in doc:
        errors.append(f"SUPPLY_CHAIN.md is missing {SECTION!r}")
    block = section(doc, "Two-build measurement (one machine)", "What is not proven")
    body = flatten(block)
    for phrase in DOC_PHRASES:
        needle = flatten(phrase).replace("*", "")
        if phrase == NOT_PROVEN:
            if phrase not in doc:
                errors.append("SUPPLY_CHAIN.md lost the not-proven statement")
        elif needle not in body:
            errors.append(f"the measurement section lost {phrase!r}")

    readme = flatten(documents[README])
    if "validate_reproducibility.py" not in readme:
        errors.append("README.md no longer indexes scripts/validate_reproducibility.py")
    workflow = documents[WORKFLOW]
    if "scripts/validate_reproducibility.py --self-test" not in workflow:
        errors.append("CI no longer self-tests the reproducibility record")
    if not re.search(r"python3 scripts/validate_reproducibility\.py\n", workflow):
        errors.append("CI no longer runs the reproducibility validator")
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
        "label": "record deleted",
        "delete": [RECORD],
        "expect": "missing supply-chain/reproducibility.json",
    },
    {
        "label": "one digest changed",
        "edit": [(RECORD, DIGEST, "0" * 64)],
        "expect": "different digest",
    },
    {
        "label": "independent environments claimed",
        "edit": [(RECORD, '"independent_environments": false', '"independent_environments": true')],
        "expect": "claims independent environments",
    },
    {
        "label": "tracker changed",
        "edit": [(RECORD, '"tracking": "#208"', '"tracking": "#999"')],
        "expect": "no longer tracks #208",
    },
    {
        "label": "measurement section renamed",
        "edit": [(DOC, SECTION, "## Two-build result")],
        "expect": "missing '## Two-build measurement (one machine)'",
    },
    {
        "label": "published digest changed",
        "replace": [(DOC, DIGEST, "1" * 64)],
        "expect": "'d7b9c351b03a1cc3b3c33c8bb526820ce99f68301ea7c397c33db59341bbeec1'",
    },
    {
        "label": "open finding claimed closed",
        "edit": [(DOC, "#208 stays OPEN", "#208 is closed")],
        "expect": "'#208 stays OPEN'",
    },
    {
        "label": "environment limit removed",
        "edit": [(DOC, "**not** the two-independent-environments comparison", "the two-independent-environments comparison")],
        "expect": "two-independent-environments comparison",
    },
    {
        "label": "not-proven statement changed",
        "edit": [(DOC, NOT_PROVEN, "Two independent builds match by hash")],
        "expect": "lost the not-proven statement",
    },
    {
        "label": "readme drops the validator",
        "edit": [(README, "validate_reproducibility.py", "validate_removed.py")],
        "expect": "no longer indexes scripts/validate_reproducibility.py",
    },
    {
        "label": "CI drops the self-test",
        "edit": [(WORKFLOW, "scripts/validate_reproducibility.py --self-test", "scripts/validate_supply_chain.py --self-test")],
        "expect": "no longer self-tests the reproducibility record",
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

    tmp = Path(tempfile.mkdtemp(prefix="curbcut-reproducibility-"))
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
        print(f"{len(errors)} reproducibility check(s) failed", file=sys.stderr)
        return 1
    print("the two-build record is published without claiming independent environments")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
