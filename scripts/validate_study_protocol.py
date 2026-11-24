#!/usr/bin/env python3
"""Fail if the study protocol loses an ethics or consent guarantee.

CC-123 requires a defined instrument and a granular, revocable consent flow,
and forbids enrolling anyone before independent ethics approval. Both are
prose, so this script checks the guarantees that matter:

* ``docs/STUDY_PROTOCOL.md`` keeps its required sections, labels every
  instrument as validated or exploratory, states that recruitment is blocked
  until ethics approval, and states the design's limits.
* ``docs/CONSENT_TEXT.md`` offers all five consent layers, each refusable and
  independently revocable, states what happens on withdrawal, promises payment
  for completed waves, and lists the content that is never collected.

Usage::

    python3 scripts/validate_study_protocol.py            # validate the repo
    python3 scripts/validate_study_protocol.py --self-test  # prove it fails

``--self-test`` copies the two documents into a temporary tree, applies one
named deletion per guarantee, and requires each to be detected.
"""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PROTOCOL = ROOT / "docs" / "STUDY_PROTOCOL.md"
CONSENT = ROOT / "docs" / "CONSENT_TEXT.md"

PROTOCOL_SECTIONS = (
    "Question and scope",
    "Participants and sample size",
    "Instrument",
    "Schedule",
    "Consent flow",
    "Privacy and data handling",
    "Analysis plan",
    "Safety and stop rules",
    "Ethics review",
    "What this study cannot show",
)

CONSENT_SECTIONS = (
    "Introduction",
    "Layered choices",
    "What happens if you stop",
    "Agreement record",
)

CONSENT_LAYERS = (
    "Study participation",
    "Clinician or carer informant",
    "Usage counters",
    "Future contact",
    "Keeping data after withdrawal",
)

NEVER_COLLECTED = ("screen content", "OCR text", "keystrokes")


def section(text: str, heading: str) -> str:
    """Return the body of a ``##`` section whose title starts with ``heading``.

    Numbered titles such as ``## 3. Instrument`` are accepted.
    """
    pattern = rf"^##\s+(?:\d+\.\s*)?{re.escape(heading)}[^\n]*$"
    match = re.search(pattern, text, re.MULTILINE)
    if match is None:
        return ""
    rest = text[match.end() :]
    following = re.search(r"^##\s+", rest, re.MULTILINE)
    return rest[: following.start()] if following else rest


def table_rows(text: str) -> list[str]:
    return [line.strip() for line in text.splitlines() if line.strip().startswith("|")]


def flatten(text: str) -> str:
    """Lowercase and collapse whitespace so a wrapped sentence still matches."""
    return " ".join(text.lower().split())


def validate_protocol(text: str) -> list[str]:
    errors: list[str] = []
    lowered = flatten(text)
    for heading in PROTOCOL_SECTIONS:
        if not re.search(
            rf"^##\s+(?:\d+\.\s*)?{re.escape(heading)}\b",
            text,
            re.IGNORECASE | re.MULTILINE,
        ):
            errors.append(f"protocol is missing the {heading} section")

    instrument = section(text, "Instrument")
    # Require the explicit disclaimer, not the word: the design section also
    # uses "exploratory" descriptively.
    if "explicitly **exploratory**" not in instrument:
        errors.append(
            "protocol does not mark the custom instrument as explicitly exploratory"
        )
    for named in ("System Usability Scale", "NASA"):
        if named.lower() not in instrument.lower():
            errors.append(f"protocol does not name the {named} instrument")

    consent = section(text, "Consent flow")
    consent_lower = flatten(consent)
    if "refusable and independently revocable" not in consent_lower:
        errors.append(
            "protocol does not state that each layer is refusable and revocable"
        )
    if "never treated as a withdrawal" not in consent_lower:
        errors.append("protocol does not protect a refusal from ending the study")
    if "withdrawal takes effect immediately" not in consent_lower:
        errors.append("protocol does not state that withdrawal is immediate")
    for layer in CONSENT_LAYERS:
        if layer.split(":")[0].lower() not in consent_lower:
            errors.append(f"protocol consent section is missing the {layer} layer")
    for item in NEVER_COLLECTED:
        if item.lower() not in consent_lower:
            errors.append(f"protocol does not forbid collecting {item!r}")

    privacy = flatten(section(text, "Privacy and data handling"))
    if "salted sha-256" not in privacy:
        errors.append("protocol does not state how identifiers are pseudonymized")
    if "salt" not in privacy or "never committed" not in privacy:
        errors.append("protocol does not state where the salt lives")

    ethics = flatten(section(text, "Ethics review"))
    if "not yet submitted" not in ethics and "pending" not in ethics:
        errors.append("protocol does not state the ethics review is outstanding")
    if "precondition" not in ethics:
        errors.append("protocol does not call ethics approval a precondition")
    if "blocked until" not in lowered:
        errors.append("protocol does not block recruitment on ethics approval")

    limits = flatten(section(text, "What this study cannot show"))
    if "no control group" not in limits:
        errors.append("protocol does not state the lack of a control group")

    # The honest-claims rule: a protocol may not assert an outcome.
    for phrase in ("curbcut works.", "proven to", "will improve", "guarantees"):
        if phrase in lowered:
            errors.append(f"protocol makes an outcome claim: {phrase!r}")
    return errors


def validate_consent(text: str) -> list[str]:
    errors: list[str] = []
    lowered = flatten(text)
    for heading in CONSENT_SECTIONS:
        if not re.search(
            rf"^##\s+(?:\d+\.\s*)?{re.escape(heading)}\b",
            text,
            re.IGNORECASE | re.MULTILINE,
        ):
            errors.append(f"consent text is missing the {heading} section")

    choices = flatten(section(text, "Layered choices"))
    if "**yes" not in choices or "**no" not in choices:
        errors.append("consent choices do not offer explicit yes and no options")
    for layer in CONSENT_LAYERS:
        if layer.lower() not in choices:
            errors.append(f"consent text is missing the {layer} choice")

    intro = flatten(section(text, "Introduction"))
    if "your choice" not in intro or "stop at any time" not in intro:
        errors.append("consent introduction does not state participation is optional")
    if "paid for each wave you complete" not in lowered:
        errors.append("consent text does not promise payment for completed waves")
    if "do not record what is on your screen" not in intro:
        errors.append("consent introduction does not state that screens are not recorded")
    if "do not record your typing" not in intro:
        errors.append("consent introduction does not state that typing is not recorded")

    stop = flatten(section(text, "What happens if you stop"))
    if "immediately" not in stop:
        errors.append("withdrawal section does not say collection stops immediately")
    if "payment" not in stop:
        errors.append("withdrawal section does not address payment")

    if not re.search(r"version:\s*\*\*\d+\.\d+", lowered):
        errors.append("consent text has no version marker")
    if "1.0" not in text:
        errors.append("consent text version is not stated")
    return errors


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    for path in (PROTOCOL, CONSENT):
        if not (root / path.relative_to(ROOT)).exists():
            errors.append(f"missing document: {path.relative_to(ROOT)}")
    if errors:
        return errors
    protocol_text = (root / PROTOCOL.relative_to(ROOT)).read_text(encoding="utf-8")
    consent_text = (root / CONSENT.relative_to(ROOT)).read_text(encoding="utf-8")
    errors.extend(f"{PROTOCOL.relative_to(ROOT)}: {m}" for m in validate_protocol(protocol_text))
    errors.extend(f"{CONSENT.relative_to(ROOT)}: {m}" for m in validate_consent(consent_text))
    return errors


def _mutated_copy(root: Path) -> Path:
    tmp = Path(tempfile.mkdtemp(prefix="curbcut-study-selftest-"))
    (tmp / "docs").mkdir()
    for source in (PROTOCOL, CONSENT):
        target = tmp / source.relative_to(ROOT)
        target.write_text(source.read_text(encoding="utf-8"), encoding="utf-8")
    return tmp


def _break(document: Path, old: str, new: str) -> None:
    text = document.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"self-test fixture no longer matches {document.name}: {old!r}")
    document.write_text(text.replace(old, new, 1), encoding="utf-8")


MUTATIONS: tuple[tuple[str, str, str, str], ...] = (
    ("protocol section", "docs/STUDY_PROTOCOL.md", "## 7. Analysis plan", "## 7. Numbers"),
    (
        "exploratory label",
        "docs/STUDY_PROTOCOL.md",
        "explicitly **exploratory**",
        "described as exploratory",
    ),
    ("validated instrument", "docs/STUDY_PROTOCOL.md", "| System Usability Scale (SUS), 10 items |", "| Usability scale, 10 items |"),
    (
        "revocability",
        "docs/STUDY_PROTOCOL.md",
        "refusable and independently revocable",
        "optional",
    ),
    (
        "refusal protection",
        "docs/STUDY_PROTOCOL.md",
        "A refusal is a valid outcome and is never treated as a withdrawal",
        "A refusal ends the study and counts as a withdrawal",
    ),
    ("withdrawal immediacy", "docs/STUDY_PROTOCOL.md", "Withdrawal takes effect immediately.", "Withdrawal is processed in due course."),
    ("salt handling", "docs/STUDY_PROTOCOL.md", "is never committed, and is", "is stored with the responses, and is"),
    ("ethics status", "docs/STUDY_PROTOCOL.md", "Status: **not yet submitted.**", "Status: **approved.**"),
    ("recruitment gate", "docs/STUDY_PROTOCOL.md", "blocked until an", "free to begin before any"),
    ("control-group limit", "docs/STUDY_PROTOCOL.md", "- No control group, so no causal claim about CurbCut's effect.", "-"),
    ("outcome claim", "docs/STUDY_PROTOCOL.md", "Primary question:", "CurbCut works. Primary question:"),
    ("consent section", "docs/CONSENT_TEXT.md", "## What happens if you stop", "## After stopping"),
    (
        "consent layer",
        "docs/CONSENT_TEXT.md",
        "### 3. Usage counters\n",
        "",
    ),
    (
        "consent layer options",
        "docs/CONSENT_TEXT.md",
        "- **Yes, collect usage counts.**\n- **No, do not collect them.**",
        "- Collect them.",
    ),
    ("optional participation", "docs/CONSENT_TEXT.md", "Taking part is your choice.", "Taking part is expected."),
    ("payment promise", "docs/CONSENT_TEXT.md", "You are\npaid for each wave you complete", "You are\nthanked for each wave you complete"),
    ("no-screen promise", "docs/CONSENT_TEXT.md", "We do not record what is on your screen.", "We may record what is on your screen."),
    ("withdrawal immediate", "docs/CONSENT_TEXT.md", "We stop collecting\nimmediately.", "We stop collecting\neventually."),
    ("never-collected keystrokes", "docs/STUDY_PROTOCOL.md", "no keystrokes,", "key logging,"),
    ("consent version", "docs/CONSENT_TEXT.md", "> Version: **1.0** — 2026-11-24.", ""),
)


def self_test() -> int:
    root = _mutated_copy(ROOT)
    try:
        if validate(root):
            print("SELF-TEST FAILED: a pristine copy did not pass", file=sys.stderr)
            return 1
        print("self-test: the committed documents pass")

        documents = sorted({root / relative for _, relative, _, _ in MUTATIONS})
        pristine = {
            document: document.read_text(encoding="utf-8") for document in documents
        }
        for label, relative, old, new in MUTATIONS:
            document = root / relative
            document.write_text(pristine[document], encoding="utf-8")
            _break(document, old, new)
            errors = validate(root)
            if not errors:
                print(
                    f"SELF-TEST FAILED: removing {label} was not detected "
                    f"({Path(relative).name})",
                    file=sys.stderr,
                )
                return 1

        (root / CONSENT.relative_to(ROOT)).unlink()
        if not validate(root):
            print("SELF-TEST FAILED: a missing document was not reported", file=sys.stderr)
            return 1

        print(
            f"self-test: {len(MUTATIONS)} named deletions were each detected, "
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
        print(f"{len(errors)} study-protocol check(s) failed", file=sys.stderr)
        return 1
    print("study protocol and consent flow keep their ethics guarantees")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
