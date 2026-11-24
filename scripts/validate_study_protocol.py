#!/usr/bin/env python3
"""Fail if the study protocol loses an ethics or consent guarantee.

CC-123 requires a defined instrument and a granular, revocable consent flow,
and forbids enrolling anyone before independent ethics approval. Both are
prose, so this script checks the guarantees that matter:

* ``docs/STUDY_PROTOCOL.md`` keeps its required sections, labels every
  instrument as validated or exploratory, states that recruitment is blocked
  until ethics approval, keeps its explicit no-outcome-claim statement, and
  states the design's limits.
* ``docs/CONSENT_TEXT.md`` offers all five consent layers, each with two
  explicit options, states what happens on withdrawal, promises payment for
  completed waves, names every layer in the agreement record, and lists the
  content that is never collected.

Usage::

    python3 scripts/validate_study_protocol.py            # validate the repo
    python3 scripts/validate_study_protocol.py --self-test  # prove it fails

``--self-test`` copies the two documents into a temporary tree, applies one
named deletion per guarantee, and requires both that validation fails *and*
that the failure names the intended check. Every case starts from pristine
bytes: restoring only the mutated document would let an earlier case's error
survive and make later cases pass for the wrong reason.
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

# Each layer must keep its affirmative and negative option. Layer five words
# them as delete/keep rather than yes/no.
LAYER_OPTIONS = {
    "Study participation": ("**yes", "**no"),
    "Clinician or carer informant": ("**yes", "**no"),
    "Usage counters": ("**yes", "**no"),
    "Future contact": ("**yes", "**no"),
    "Keeping data after withdrawal": ("**delete", "**keep"),
}

# (phrase that must survive, label used in the error)
NEVER_COLLECTED = (
    ("no screen content", "screen content"),
    ("no ocr text", "OCR text"),
    ("no keystrokes", "keystrokes"),
)

OUTCOME_PHRASES = ("proven to", "will improve", "guarantees", "is effective")


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


def subsection(text: str, heading: str) -> str:
    """Return the body of a ``###`` block whose title starts with ``heading``."""
    pattern = rf"^###\s+(?:\d+\.\s*)?{re.escape(heading)}[^\n]*$"
    match = re.search(pattern, text, re.MULTILINE)
    if match is None:
        return ""
    rest = text[match.end() :]
    following = re.search(r"^###\s+", rest, re.MULTILINE)
    return rest[: following.start()] if following else rest


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

    consent = flatten(section(text, "Consent flow"))
    if "refusable and independently revocable" not in consent:
        errors.append(
            "protocol does not state that each layer is refusable and revocable"
        )
    if "never treated as a withdrawal" not in consent:
        errors.append("protocol does not protect a refusal from ending the study")
    if "withdrawal takes effect immediately" not in consent:
        errors.append("protocol does not state that withdrawal is immediate")
    for layer in CONSENT_LAYERS:
        if layer.lower() not in consent:
            errors.append(f"protocol consent section is missing the {layer} layer")
    for phrase, label in NEVER_COLLECTED:
        if phrase not in consent:
            errors.append(f"protocol does not forbid collecting {label!r}")

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

    # An outcome claim is banned outright. The disclaimer is checked separately,
    # and removed before the scan, because an honest sentence may legitimately
    # contain a phrase such as "CurbCut works" while denying it.
    denial = "nothing here says curbcut works"
    if denial not in lowered:
        errors.append("protocol lost its explicit no-outcome-claim statement")
    claims = lowered.replace(denial, "")
    for phrase in ("curbcut works",) + OUTCOME_PHRASES:
        if phrase in claims:
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
    for layer, options in LAYER_OPTIONS.items():
        block = flatten(subsection(text, layer))
        if layer.lower() not in choices:
            errors.append(f"consent text is missing the {layer} choice")
        if not all(option in block for option in options):
            errors.append(f"consent layer {layer} does not offer both explicit options")

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

    record = flatten(section(text, "Agreement record"))
    for layer in CONSENT_LAYERS:
        if layer.lower() not in record:
            errors.append(f"agreement record does not name the {layer} choice")

    marker = re.search(r"version:\s*\*\*(?P<version>\d+\.\d+)\*\*", lowered)
    if marker is None:
        errors.append("consent text has no version marker")
    elif marker.group("version") not in section(text, "Agreement record"):
        errors.append("agreement record does not state the stored version")
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


# (label, document, find, replace, substring of the error this must produce)
Mutation = tuple[str, str, str, str, str]

MUTATIONS: tuple[Mutation, ...] = (
    (
        "protocol section",
        "docs/STUDY_PROTOCOL.md",
        "## 7. Analysis plan",
        "## 7. Numbers",
        "protocol is missing the Analysis plan section",
    ),
    (
        "instrument section",
        "docs/STUDY_PROTOCOL.md",
        "## 3. Instrument",
        "## 3. Tools",
        "protocol is missing the Instrument section",
    ),
    (
        "ethics section",
        "docs/STUDY_PROTOCOL.md",
        "## 9. Ethics review",
        "## 9. Review",
        "protocol is missing the Ethics review section",
    ),
    (
        "exploratory label",
        "docs/STUDY_PROTOCOL.md",
        "explicitly **exploratory**",
        "described as exploratory",
        "does not mark the custom instrument as explicitly exploratory",
    ),
    (
        "validated SUS",
        "docs/STUDY_PROTOCOL.md",
        "| System Usability Scale (SUS), 10 items |",
        "| Usability scale, 10 items |",
        "does not name the System Usability Scale instrument",
    ),
    (
        "validated NASA",
        "docs/STUDY_PROTOCOL.md",
        "| NASA Task Load Index (NASA-TLX), 6 items |",
        "| Task load index, 6 items |",
        "does not name the NASA instrument",
    ),
    (
        "revocability",
        "docs/STUDY_PROTOCOL.md",
        "refusable and independently revocable",
        "optional",
        "does not state that each layer is refusable and revocable",
    ),
    (
        "refusal protection",
        "docs/STUDY_PROTOCOL.md",
        "A refusal is a valid outcome and is never treated as a withdrawal",
        "A refusal ends the study and counts as a withdrawal",
        "does not protect a refusal from ending the study",
    ),
    (
        "withdrawal immediacy",
        "docs/STUDY_PROTOCOL.md",
        "Withdrawal takes effect immediately.",
        "Withdrawal is processed in due course.",
        "does not state that withdrawal is immediate",
    ),
    (
        "consent layer row",
        "docs/STUDY_PROTOCOL.md",
        "| Usage counters | Collect counts-only product telemetry | Yes | Yes, independently |\n",
        "",
        "consent section is missing the Usage counters layer",
    ),
    (
        "screen-content forbiddance",
        "docs/STUDY_PROTOCOL.md",
        "No screen content, no OCR text,",
        "Screen content and OCR text,",
        "does not forbid collecting 'screen content'",
    ),
    (
        "OCR-text forbiddance",
        "docs/STUDY_PROTOCOL.md",
        "no OCR text, no accessibility-tree strings",
        "OCR text and accessibility-tree strings",
        "does not forbid collecting 'OCR text'",
    ),
    (
        "keystroke forbiddance",
        "docs/STUDY_PROTOCOL.md",
        "no keystrokes,",
        "key logging,",
        "does not forbid collecting 'keystrokes'",
    ),
    (
        "pseudonymization method",
        "docs/STUDY_PROTOCOL.md",
        "reduced to a salted SHA-256 pseudonym",
        "reduced to a pseudonym",
        "does not state how identifiers are pseudonymized",
    ),
    (
        "salt handling",
        "docs/STUDY_PROTOCOL.md",
        "is never committed, and is",
        "is stored with the responses, and is",
        "does not state where the salt lives",
    ),
    (
        "ethics status",
        "docs/STUDY_PROTOCOL.md",
        "Status: **not yet submitted.**",
        "Status: **approved.**",
        "does not state the ethics review is outstanding",
    ),
    (
        "precondition",
        "docs/STUDY_PROTOCOL.md",
        "Approval\nis a precondition, not a formality:",
        "Enrollment\nmay begin regardless:",
        "does not call ethics approval a precondition",
    ),
    (
        "recruitment gate",
        "docs/STUDY_PROTOCOL.md",
        "Recruitment is blocked until an",
        "Recruitment is free to begin before any",
        "does not block recruitment on ethics approval",
    ),
    (
        "control-group limit",
        "docs/STUDY_PROTOCOL.md",
        "- No control group, so no causal claim about CurbCut's effect.",
        "-",
        "does not state the lack of a control group",
    ),
    (
        "outcome phrase",
        "docs/STUDY_PROTOCOL.md",
        "Pre-registered before recruitment, and published with the protocol:",
        "This protocol shows CurbCut is proven to work. Pre-registered before "
        "recruitment, and published with the protocol:",
        "makes an outcome claim",
    ),
    (
        "bare outcome claim",
        "docs/STUDY_PROTOCOL.md",
        "Primary question: for adults with cognitive or learning disabilities, does",
        "CurbCut works. Primary question: for adults with cognitive or learning "
        "disabilities, does",
        "makes an outcome claim",
    ),
    (
        "non-claim statement",
        "docs/STUDY_PROTOCOL.md",
        "Nothing here says CurbCut works",
        "CurbCut works",
        "lost its explicit no-outcome-claim statement",
    ),
    (
        "consent section",
        "docs/CONSENT_TEXT.md",
        "## What happens if you stop",
        "## After stopping",
        "consent text is missing the What happens if you stop section",
    ),
    (
        "agreement section",
        "docs/CONSENT_TEXT.md",
        "## Agreement record",
        "## Record",
        "consent text is missing the Agreement record section",
    ),
    (
        "consent layer",
        "docs/CONSENT_TEXT.md",
        "### 3. Usage counters\n",
        "",
        "consent text is missing the Usage counters choice",
    ),
    (
        "single-layer options",
        "docs/CONSENT_TEXT.md",
        "- **Yes, collect usage counts.**\n- **No, do not collect them.**",
        "- Collect them.",
        "consent layer Usage counters does not offer both explicit options",
    ),
    (
        "withdrawal-layer options",
        "docs/CONSENT_TEXT.md",
        "- **Delete everything I have given.**\n"
        "- **Keep it as anonymous totals that cannot be traced to me.**",
        "- **Delete everything I have given.**",
        "consent layer Keeping data after withdrawal does not offer both "
        "explicit options",
    ),
    (
        "optional participation",
        "docs/CONSENT_TEXT.md",
        "Taking part is your choice.",
        "Taking part is expected.",
        "does not state participation is optional",
    ),
    (
        "payment promise",
        "docs/CONSENT_TEXT.md",
        "You are\npaid for each wave you complete",
        "You are\nthanked for each wave you complete",
        "does not promise payment for completed waves",
    ),
    (
        "no-screen promise",
        "docs/CONSENT_TEXT.md",
        "We do not record what is on your screen.",
        "We may record what is on your screen.",
        "does not state that screens are not recorded",
    ),
    (
        "typing promise",
        "docs/CONSENT_TEXT.md",
        "We do not record your typing.",
        "We may record your typing.",
        "does not state that typing is not recorded",
    ),
    (
        "withdrawal immediate",
        "docs/CONSENT_TEXT.md",
        "We stop collecting\nimmediately.",
        "We stop collecting\neventually.",
        "does not say collection stops immediately",
    ),
    (
        "withdrawal payment",
        "docs/CONSENT_TEXT.md",
        "You keep payment for waves you finished.",
        "You keep our thanks for waves you finished.",
        "withdrawal section does not address payment",
    ),
    (
        "agreement layer name",
        "docs/CONSENT_TEXT.md",
        "- Clinician or carer informant: yes / no",
        "- Informant: yes / no",
        "agreement record does not name the Clinician or carer informant choice",
    ),
    (
        "version marker",
        "docs/CONSENT_TEXT.md",
        "> Version: **1.0** — 2026-11-24.",
        "",
        "has no version marker",
    ),
    (
        "stored version",
        "docs/CONSENT_TEXT.md",
        "we store the version (1.0)",
        "we store the version",
        "does not state the stored version",
    ),
)


def self_test() -> int:
    root = _mutated_copy(ROOT)
    try:
        if validate(root):
            print("SELF-TEST FAILED: a pristine copy did not pass", file=sys.stderr)
            return 1
        print("self-test: the committed documents pass")

        documents = sorted({root / relative for _, relative, _, _, _ in MUTATIONS})
        pristine = {
            document: document.read_text(encoding="utf-8") for document in documents
        }
        for label, relative, old, new, expected in MUTATIONS:
            # Restore every document, not just the one under test. A leftover
            # edit from an earlier case would otherwise be the reason a later
            # case passes.
            for document in documents:
                document.write_text(pristine[document], encoding="utf-8")
            _break(root / relative, old, new)
            errors = validate(root)
            matched = [error for error in errors if expected in error]
            if not matched:
                print(
                    f"SELF-TEST FAILED: removing {label} was not detected by its "
                    f"own check ({Path(relative).name}); wanted {expected!r}, "
                    f"got {errors[:2]}",
                    file=sys.stderr,
                )
                return 1

        (root / CONSENT.relative_to(ROOT)).unlink()
        if not validate(root):
            print("SELF-TEST FAILED: a missing document was not reported", file=sys.stderr)
            return 1

        print(
            f"self-test: {len(MUTATIONS)} named deletions were each detected by "
            "their own check, and a missing document failed"
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
