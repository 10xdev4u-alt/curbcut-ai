#!/usr/bin/env python3
"""Fail if the voice-path document drifts into a retention overclaim.

Issue #201 asked for an end-to-end description of the microphone path and for
the sub-second bound to be attributed. No code enforces that bound today: there
is no capture, audio buffer, or ASR call in the workspace. This validator
checks that the canonical document and its consumers keep saying so, that
F-06 and DPIA R-04 stay open, and that no clock rule is invented in the
push-to-talk gate.

Run ``--self-test`` to apply every named mutation to a pristine copy and
require detection. Every mutation source is preflighted against the committed
files, so a stale fixture fails cleanly instead of silently passing.
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

VOICE = "docs/VOICE_PIPELINE.md"
INVENTORY = "docs/DATA_INVENTORY.md"
THREAT = "docs/THREAT_MODEL.md"
MODELS = "docs/MODELS.md"
SPEECH = "docs/SPEECH.md"
VOICE_COMMANDS = "docs/VOICE_COMMANDS.md"
DPIA = "docs/DPIA.md"
FINDINGS = "docs/PRIVACY_FINDINGS.md"
COMMAND_RS = "crates/curbcut-tts/src/command.rs"
README = "README.md"
WORKFLOW = ".github/workflows/ci.yml"

DOCUMENTS = (
    VOICE,
    INVENTORY,
    THREAT,
    MODELS,
    SPEECH,
    VOICE_COMMANDS,
    DPIA,
    FINDINGS,
    COMMAND_RS,
    README,
    WORKFLOW,
)

CONSUMERS = {
    INVENTORY: "not implemented today",
    THREAT: "no code enforces",
    MODELS: "planned",
    SPEECH: "planned",
    VOICE_COMMANDS: "planned",
    DPIA: "unenforced",
}

SECTIONS = (
    "Status",
    "The path",
    "What is held in RAM",
    "Who enforces the bound",
    "Never stored or logged",
    "Open work and tracking",
)

REQUIRED_PHRASES = (
    "< 1 s",
    "<1-second",
    "planned",
    "no code enforces",
    "asr-v1",
    "dc-32",
    "dc-33",
    "dc-34",
    "f-06",
    "r-04",
    "securebuffer",
    "raw samples are never stored",
    "#217",
    "production caller",
    "sample_rate_hz",
    "retention test",
)

OVERCLAIMS = (
    "enforced today",
    "is enforced",
    "are enforced",
    "wcag compliant",
    "ada compliant",
    "makes apps accessible",
)

CLOCK_PRIMITIVE = re.compile(r"Instant|SystemTime|Duration|std::time|sleep")

# A flipped DC-32/DC-33 row is only credible once the #217 checklist names the
# production module that owns the audio window. The placeholder below is not a
# module; a real one is a crates/... path on the "Production module:" line.
PRODUCTION_MODULE = re.compile(
    r"(?m)^[ \t]*Production module:[ \t]*`crates/[A-Za-z0-9_./-]+\.rs`[ \t]*$"
)

# Exact #217 block in docs/VOICE_PIPELINE.md; also the mutation source for
# removing the whole checklist. Preflighted with every other mutation source.
CHECKLIST = """Evidence checklist for #217 (all three items are required before any
implementation claim):

1. **Production caller.** A named production caller in a capture adapter that
   owns the audio window; a standalone capped-audio type with no caller is not
   evidence.
   Production module: none named today.
2. **Frame budget.** The bound is expressed as a sample-rate-derived frame
   budget from `sample_rate_hz`, not a wall clock.
3. **Retention test.** A retention test fails when more than
   `sample_rate_hz - 1` frames are retained.

Until all three items exist, F-06 and R-04 stay open and no implementation
claim is allowed."""


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

    voice = documents[VOICE]
    for heading in SECTIONS:
        if not re.search(rf"^#{{1,6}}\s+{re.escape(heading)}\b", voice, re.MULTILINE):
            errors.append(f"VOICE_PIPELINE.md is missing the {heading!r} section")
    body = flatten(voice)
    for phrase in REQUIRED_PHRASES:
        if phrase not in body:
            errors.append(f"VOICE_PIPELINE.md no longer states {phrase!r}")
    for phrase in OVERCLAIMS:
        if phrase in body:
            errors.append(f"VOICE_PIPELINE.md overclaims {phrase!r}")

    for relative, phrase in CONSUMERS.items():
        text = documents[relative]
        if "](VOICE_PIPELINE.md)" not in text:
            errors.append(f"{relative} no longer links to docs/VOICE_PIPELINE.md")
        if phrase not in flatten(text):
            errors.append(f"{relative} no longer states {phrase!r}")

    for dc, expected in (("DC-32", "planned"), ("DC-33", "planned"), ("DC-34", "in-memory")):
        row = table_cells(documents[INVENTORY], dc)
        if row is None:
            errors.append(f"DATA_INVENTORY.md has no {dc} row")
        elif len(row) < 10:
            errors.append(f"{dc} row is truncated")
        elif row[9] != expected:
            errors.append(f"{dc} code status is {row[9]!r}, not {expected!r}")

    flipped = False
    for dc in ("DC-32", "DC-33"):
        row = table_cells(documents[INVENTORY], dc)
        if row is not None and len(row) >= 10 and row[9] != "planned":
            flipped = True
    module_match = PRODUCTION_MODULE.search(voice)
    if "Production module: none named today." not in voice and module_match is None:
        errors.append(
            "VOICE_PIPELINE.md Production module line is neither the none "
            "sentinel nor a crates path"
        )
    if flipped:
        if module_match is None:
            errors.append("DC-32/DC-33 cannot be implemented without #217 evidence")
        else:
            relative = module_match.group(0).split("`")[1]
            if ".." in relative or not (root / relative).is_file():
                errors.append(
                    "DC-32/DC-33 evidence names a nonexistent production module"
                )

    threat_row = row_containing(documents[THREAT], "| Voice audio |")
    if not threat_row:
        errors.append("THREAT_MODEL.md has no voice audio row")
    elif "< 1 s" not in threat_row:
        errors.append("THREAT_MODEL.md voice row lost the < 1 s bound")

    r04 = table_cells(documents[DPIA], "R-04")
    if r04 is None:
        errors.append("DPIA.md has no R-04 row")
    elif len(r04) < 5:
        errors.append("R-04 row is truncated")
    elif "unenforced" not in flatten(r04[4]):
        errors.append("R-04 no longer states the bound is unenforced")
    if "#217" not in documents[DPIA]:
        errors.append("DPIA.md no longer tracks enforcement in #217")
    if "real until #217" not in flatten(documents[DPIA]):
        errors.append("DPIA residual risk no longer tracks #217")

    f06 = table_cells(documents[FINDINGS], "F-06")
    if f06 is None:
        errors.append("PRIVACY_FINDINGS.md has no F-06 row")
    elif len(f06) < 3:
        errors.append("F-06 row is truncated")
    elif f06[2] != "OPEN":
        errors.append(f"F-06 status is {f06[2]!r}, not OPEN")
    elif f06[-1].strip() != "#217":
        errors.append("F-06 no longer tracks enforcement in #217")

    match = CLOCK_PRIMITIVE.search(documents[COMMAND_RS])
    if match:
        errors.append(f"command.rs invents a clock rule ({match.group(0)})")

    readme = flatten(documents[README])
    if "validate_voice_pipeline.py" not in readme:
        errors.append("README.md no longer indexes scripts/validate_voice_pipeline.py")
    workflow = documents[WORKFLOW]
    if "scripts/validate_voice_pipeline.py --self-test" not in workflow:
        errors.append("CI no longer self-tests the voice-path validator")
    if not re.search(r"python3 scripts/validate_voice_pipeline\.py\n", workflow):
        errors.append("CI no longer runs the voice-path validator")
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
        "delete": [VOICE],
        "expect": "missing docs/VOICE_PIPELINE.md",
    },
    {
        "label": "status section renamed",
        "edit": [(VOICE, "## Status", "## Current state")],
        "expect": "'Status' section",
    },
    {
        "label": "enforcement invented",
        "edit": [
            (VOICE, "No code enforces the bound today.", "The bound is enforced today.")
        ],
        "expect": "overclaims 'enforced today'",
    },
    {
        "label": "bound inflated",
        "replace": [(VOICE, "`< 1 s`", "`< 10 s`")],
        "expect": "'< 1 s'",
    },
    {
        "label": "DC-32 marked implemented",
        "edit": [
            (
                INVENTORY,
                "| mic permission, voice control off by default | local-only | yes | planned |",
                "| mic permission, voice control off by default | local-only | yes | implemented |",
            )
        ],
        "expect": "DC-32 code status",
    },
    {
        "label": "threat model bound removed",
        "edit": [(THREAT, "| Voice audio | RAM | < 1 s |", "| Voice audio | RAM | session |")],
        "expect": "voice row lost the < 1 s bound",
    },
    {
        "label": "DC-33 marked implemented",
        "edit": [
            (
                INVENTORY,
                "| mic permission for recognition | local-only | yes | planned |",
                "| mic permission for recognition | local-only | yes | implemented |",
            )
        ],
        "expect": "DC-33 code status",
    },
    {
        "label": "DC-34 marked planned",
        "edit": [
            (
                INVENTORY,
                "| push to talk, utterance not retained | local-only | yes | in-memory |",
                "| push to talk, utterance not retained | local-only | yes | planned |",
            )
        ],
        "expect": "DC-34 code status",
    },
    {
        "label": "speech loses its planned wording",
        "edit": [(SPEECH, "is planned and separate; see", "is implemented; see")],
        "expect": "SPEECH.md no longer states 'planned'",
    },
    {
        "label": "a clock rule appears in push-to-talk",
        "edit": [(COMMAND_RS, "pub enum PushToTalkMode {", "pub enum PushToTalkMode { // Duration")],
        "expect": "invents a clock rule",
    },
    {
        "label": "residual risk drops the tracker",
        "edit": [(DPIA, "Real\n  until #217 implements", "Real\n  until enforcement lands")],
        "expect": "DPIA residual risk no longer tracks #217",
    },
    {
        "label": "voice commands drops the link",
        "edit": [(VOICE_COMMANDS, "[`docs/VOICE_PIPELINE.md`](VOICE_PIPELINE.md)", "the pipeline doc")],
        "expect": "VOICE_COMMANDS.md no longer links",
    },
    {
        "label": "speech drops the link",
        "edit": [(SPEECH, "[`docs/VOICE_PIPELINE.md`](VOICE_PIPELINE.md)", "the pipeline doc")],
        "expect": "SPEECH.md no longer links",
    },
    {
        "label": "R-04 calls the bound enforced",
        "edit": [(DPIA, "**unenforced**, no capture code today", "**enforced**")],
        "expect": "R-04 no longer states the bound is unenforced",
    },
    {
        "label": "finding closed early",
        "edit": [(FINDINGS, "| F-06 | HIGH | OPEN |", "| F-06 | HIGH | MITIGATED |")],
        "expect": "F-06 status",
    },
    {
        "label": "readme drops the validator",
        "edit": [(README, "validate_voice_pipeline.py", "validate_removed.py")],
        "expect": "no longer indexes scripts/validate_voice_pipeline.py",
    },
    {
        "label": "CI drops the self-test",
        "edit": [
            (WORKFLOW, "scripts/validate_voice_pipeline.py --self-test", "scripts/validate_hashes.py --self-test")
        ],
        "expect": "no longer self-tests the voice-path validator",
    },
    {
        "label": "evidence checklist removed",
        "edit": [(VOICE, CHECKLIST, "")],
        "expect": "VOICE_PIPELINE.md no longer states 'production caller'",
    },
    {
        "label": "frame budget replaced with a wall clock",
        "replace": [(VOICE, "sample_rate_hz", "Instant")],
        "expect": "VOICE_PIPELINE.md no longer states 'sample_rate_hz'",
    },
    {
        "label": "production module sentinel drifted",
        "edit": [(VOICE, "Production module: none named today.", "Production module: soon")],
        "expect": "Production module line is neither",
    },
    {
        "label": "fake production module with a flipped status",
        "edit": [
            (
                INVENTORY,
                "| mic permission, voice control off by default | local-only | yes | planned |",
                "| mic permission, voice control off by default | local-only | yes | implemented |",
            ),
            (
                VOICE,
                "Production module: none named today.",
                "Production module: `crates/curbcut-audio/src/capture.rs`",
            ),
        ],
        "expect": "nonexistent production module",
    },
    {
        "label": "DC-32 flipped without #217 evidence",
        "edit": [
            (
                INVENTORY,
                "| mic permission, voice control off by default | local-only | yes | planned |",
                "| mic permission, voice control off by default | local-only | yes | implemented |",
            )
        ],
        "expect": "DC-32/DC-33 cannot be implemented without #217 evidence",
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
    # A stale fixture used to surface as a traceback three laps in a row.
    # Check every mutation source before touching any file.
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

    tmp = Path(tempfile.mkdtemp(prefix="curbcut-voice-"))
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
        print(f"{len(errors)} voice-path check(s) failed", file=sys.stderr)
        return 1
    print("the voice path is documented without retention overclaims")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
