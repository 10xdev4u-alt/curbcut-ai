#!/usr/bin/env python3
"""Fail if an until-deleted class loses its control statement or review cadence.

Issue #198 found six classes recorded as "until deleted" with no cadence, no
maximum lifetime, and no documented deletion affordance. `docs/DATA_INVENTORY.md`
§Until-deleted classes now names each class's code-level control or the manual
path owed when persistence ships, and the DPIA sets a review cadence. This
validator checks that the table, cross-references, findings register, and
code-level guards stay consistent. F-10 stays OPEN: documentation is not a
deletion control.

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

INVENTORY = "docs/DATA_INVENTORY.md"
DPIA = "docs/DPIA.md"
FINDINGS = "docs/PRIVACY_FINDINGS.md"
PROFILE_DOC = "docs/PROFILE_GRAPH.md"
LAYOUT_DOC = "docs/LAYOUT_MEMORY.md"
FEEDBACK_DOC = "docs/LEARNED_SIMPLIFICATIONS.md"
INBOX_DOC = "docs/INTERRUPTION_INBOX.md"
TRANSMISSION_DOC = "docs/TRANSMISSION_PATHS.md"
CLI_RS = "crates/curbcut-cli/src/command.rs"
CLI_DOC = "docs/CLI.md"
PROFILE_RS = "crates/curbcut-adapt/src/profile.rs"
LAYOUT_RS = "crates/curbcut-store/src/layout_memory.rs"
FEEDBACK_RS = "crates/curbcut-store/src/feedback.rs"
INBOX_RS = "crates/curbcut-adapt/src/inbox.rs"
STORE_RS = "crates/curbcut-store/src/lib.rs"
NET_RS = "crates/curbcut-core/src/net.rs"
README = "README.md"
WORKFLOW = ".github/workflows/ci.yml"

DOCUMENTS = (
    INVENTORY,
    DPIA,
    FINDINGS,
    PROFILE_DOC,
    LAYOUT_DOC,
    FEEDBACK_DOC,
    INBOX_DOC,
    TRANSMISSION_DOC,
    CLI_RS,
    CLI_DOC,
    PROFILE_RS,
    LAYOUT_RS,
    FEEDBACK_RS,
    INBOX_RS,
    STORE_RS,
    NET_RS,
    README,
    WORKFLOW,
)

HEADER = "| Class | Declared rule | Deletion control today | Review cadence | Finding |"
CLASSES = {
    "Profiles (DC-13)": ("no delete api", "reset", "manually"),
    "Per-app layouts (DC-14)": (
        "reset_app",
        "reset_all",
        "not wired",
        "manually",
        "forget-layouts",
    ),
    "Learned simplifications (DC-15)": ("clear", "not wired", "manually"),
    "Interruption inbox (DC-24)": ("remove", "clear", "ram", "no file"),
    "Backup and encrypted export (DC-28)": ("no export", "manually"),
    "Optional encrypted sync (DC-29)": ("no channel", "nothing is synced", "owed"),
}
CONSUMERS = {
    PROFILE_DOC: "DC-13",
    LAYOUT_DOC: "DC-14",
    FEEDBACK_DOC: "DC-15",
    INBOX_DOC: "DC-24",
    TRANSMISSION_DOC: "DC-28",
}

OVERCLAIMS = (
    "compliant",
    "compliance",
    "wcag",
    "ada",
    "gdpr",
    "certified",
    "guaranteed",
    "automatic",
    "automatically",
    "expire",
    "expiry",
    "purge",
    "enforced",
    "makes apps accessible",
)


def flatten(text: str) -> str:
    return " ".join(text.lower().replace("`", "").split())


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
    if not re.search(r"^##\s+Until-deleted classes\b", inventory, re.MULTILINE):
        errors.append("DATA_INVENTORY.md is missing the 'Until-deleted classes' section")
    block = section(inventory, "Until-deleted classes", "What this inventory does not cover")
    if HEADER not in block:
        errors.append("the until-deleted table header changed")
    for phrase in (
        "no maximum lifetime",
        "f-10 stays open",
        "no user-facing deletion control",
        "process-local",
    ):
        if phrase not in flatten(block):
            errors.append(f"the until-deleted section lost {phrase!r}")
    for phrase in OVERCLAIMS:
        if phrase in flatten(block):
            errors.append(f"the until-deleted section overclaims {phrase!r}")

    for label, phrases in CLASSES.items():
        cells = table_cells(block, label)
        if cells is None:
            errors.append(f"no until-deleted row for {label}")
            continue
        if len(cells) < 5:
            errors.append(f"the {label} row is truncated")
            continue
        if cells[1] != "until-deleted":
            errors.append(f"{label} declared rule is {cells[1]!r}")
        if cells[3] != "Each DPIA review":
            errors.append(f"{label} review cadence is not 'Each DPIA review'")
        if cells[4] != "F-10":
            errors.append(f"{label} no longer points at F-10")
        control = flatten(cells[2])
        for phrase in phrases:
            if phrase not in control:
                errors.append(f"{label} lost its control statement ({phrase!r})")
        dc = re.search(r"DC-\d+", label).group(0)
        row = table_cells(inventory, dc)
        if row is None:
            errors.append(f"inventory has no {dc} row")
        elif len(row) < 5:
            errors.append(f"inventory {dc} row is truncated")
        elif row[4] != "until-deleted":
            errors.append(f"inventory {dc} retention is {row[4]!r}, not 'until-deleted'")

    dpia = documents[DPIA]
    if "- **Until-deleted review:**" not in dpia:
        errors.append("DPIA has no until-deleted review bullet")
    for phrase in ("§until-deleted classes", "f-10 stays open"):
        if phrase not in flatten(dpia):
            errors.append(f"DPIA lost {phrase!r}")

    for relative, dc in CONSUMERS.items():
        text = documents[relative]
        if "](DATA_INVENTORY.md)" not in text:
            errors.append(f"{relative} no longer references DATA_INVENTORY.md")
        body = flatten(text)
        if "until-deleted classes" not in body:
            errors.append(f"{relative} no longer references Until-deleted classes")
        if dc.lower() not in body:
            errors.append(f"{relative} no longer mentions {dc}")

    cli_rs = documents[CLI_RS]
    if '"forget-layouts" => Ok(Self::ForgetLayouts)' not in cli_rs:
        errors.append("CLI no longer parses forget-layouts")
    cli_doc = flatten(documents[CLI_DOC])
    if "forget-layouts" not in cli_doc:
        errors.append("CLI.md no longer documents forget-layouts")
    if "process-local, ram-only" not in cli_doc:
        errors.append("CLI.md no longer states the deletion is process-local")

    f10 = table_cells(documents[FINDINGS], "F-10")
    if f10 is None:
        errors.append("PRIVACY_FINDINGS.md has no F-10 row")
    elif len(f10) < 3:
        errors.append("F-10 row is truncated")
    elif f10[2] != "OPEN":
        errors.append(f"F-10 status is {f10[2]!r}, not OPEN")
    elif f10[-1].strip() != "#198":
        errors.append("F-10 no longer tracks #198")

    profile = documents[PROFILE_RS]
    if "pub fn reset(&mut self) -> Vec<PreviewEvent>" not in profile:
        errors.append("profile.rs lost ProfileGraph::reset")
    if "pub fn delete" in profile:
        errors.append("profile.rs invents a delete API")
    layout = documents[LAYOUT_RS]
    for phrase in ("pub fn forget(&mut self, key: &LayoutKey)", "pub fn reset_app", "pub fn reset_user", "pub fn reset_all"):
        if phrase not in layout:
            errors.append(f"layout_memory.rs lost {phrase!r}")
    feedback = documents[FEEDBACK_RS]
    if feedback.count("pub fn clear(&mut self)") < 2:
        errors.append("feedback.rs lost a clear method")
    inbox = documents[INBOX_RS]
    for phrase in ("pub fn remove(&mut self, id: InboxId)", "pub fn clear(&mut self)"):
        if phrase not in inbox:
            errors.append(f"inbox.rs lost {phrase!r}")
    store = documents[STORE_RS]
    if "pub mod backup" in store or "pub mod sync" in store:
        errors.append("store now has an unrecorded backup or sync module")
    net = documents[NET_RS]
    if "Sync," not in net or "ALL: [NetworkFeature; 5]" not in net:
        errors.append("net.rs no longer matches the sync-flag-only guard")

    readme = flatten(documents[README])
    if "validate_until_deleted.py" not in readme:
        errors.append("README.md no longer indexes scripts/validate_until_deleted.py")
    workflow = documents[WORKFLOW]
    if "scripts/validate_until_deleted.py --self-test" not in workflow:
        errors.append("CI no longer self-tests the until-deleted validator")
    if not re.search(r"python3 scripts/validate_until_deleted\.py\n", workflow):
        errors.append("CI no longer runs the until-deleted validator")
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
        "label": "section heading renamed",
        "edit": [(INVENTORY, "## Until-deleted classes", "## Deleted classes")],
        "expect": "missing the 'Until-deleted classes' section",
    },
    {
        "label": "table header changed",
        "edit": [(INVENTORY, HEADER, "| Class | Rule | Control | Cadence | Finding |")],
        "expect": "until-deleted table header changed",
    },
    {
        "label": "DC-14 row deleted",
        "edit": [(INVENTORY, "| Per-app layouts (DC-14) | until-deleted |", "| Per-app layouts (DC-14) | session |")],
        "expect": "declared rule is 'session'",
    },
    {
        "label": "DC-13 rule changed",
        "edit": [(INVENTORY, "| Profiles (DC-13) | until-deleted |", "| Profiles (DC-13) | 90-days |")],
        "expect": "declared rule is '90-days'",
    },
    {
        "label": "DC-24 control statement lost",
        "edit": [(INVENTORY, "no file to delete", "a file to delete")],
        "expect": "lost its control statement",
    },
    {
        "label": "cadence lost",
        "edit": [(INVENTORY, "| Each DPIA review | F-10 |", "| never | F-10 |")],
        "expect": "review cadence is not 'Each DPIA review'",
    },
    {
        "label": "finding changed",
        "edit": [(INVENTORY, "| Each DPIA review | F-10 |", "| Each DPIA review | F-11 |")],
        "expect": "no longer points at F-10",
    },
    {
        "label": "no-maximum-lifetime statement lost",
        "edit": [(INVENTORY, "There is **no maximum lifetime**", "There is a maximum lifetime")],
        "expect": "'no maximum lifetime'",
    },
    {
        "label": "automatic deletion overclaim",
        "edit": [(INVENTORY, "**F-10 stays OPEN**", "The app automatically deletes these records; **F-10 stays OPEN**")],
        "expect": "overclaims 'automatic'",
    },
    {
        "label": "DPIA review bullet deleted",
        "edit": [(DPIA, "- **Until-deleted review:**", "- **Review of deleted rows:**")],
        "expect": "DPIA has no until-deleted review bullet",
    },
    {
        "label": "profile doc drops the cross-reference",
        "edit": [(PROFILE_DOC, "[`docs/DATA_INVENTORY.md`](DATA_INVENTORY.md)", "the inventory")],
        "expect": "PROFILE_GRAPH.md no longer references DATA_INVENTORY.md",
    },
    {
        "label": "layout control renamed",
        "edit": [(LAYOUT_RS, "pub fn reset_all(&mut self) -> usize", "pub fn reset_everything(&mut self) -> usize")],
        "expect": "layout_memory.rs lost 'pub fn reset_all'",
    },
    {
        "label": "a delete API appears",
        "edit": [(PROFILE_RS, "pub fn reset(&mut self) -> Vec<PreviewEvent>", "pub fn delete(&mut self) -> Vec<PreviewEvent>")],
        "expect": "profile.rs invents a delete API",
    },
    {
        "label": "CLI forget command removed",
        "edit": [(CLI_RS, '"forget-layouts" => Ok(Self::ForgetLayouts)', '"forget-everything" => Ok(Self::ForgetLayouts)')],
        "expect": "CLI no longer parses forget-layouts",
    },
    {
        "label": "readme drops the validator",
        "edit": [(README, "validate_until_deleted.py", "validate_removed.py")],
        "expect": "no longer indexes scripts/validate_until_deleted.py",
    },
    {
        "label": "CI drops the self-test",
        "edit": [(WORKFLOW, "scripts/validate_until_deleted.py --self-test", "scripts/validate_network_policy.py --self-test")],
        "expect": "no longer self-tests the until-deleted validator",
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


def self_test() -> int:
    pristine = {
        relative: (ROOT / relative).read_text(encoding="utf-8")
        for relative in collected_files(ROOT)
    }
    for case in CASES:
        for relative, old, _ in case.get("edit", ()):  # type: ignore[union-attr]
            if old not in pristine[str(relative)]:
                print(
                    f"SELF-TEST FAILED: stale fixture in {case['label']}: "
                    f"{old!r} not in {relative}",
                    file=sys.stderr,
                )
                return 1

    tmp = Path(tempfile.mkdtemp(prefix="curbcut-until-deleted-"))
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
        print(f"{len(errors)} until-deleted check(s) failed", file=sys.stderr)
        return 1
    print("each until-deleted class names a control or a manual path and is reviewed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
