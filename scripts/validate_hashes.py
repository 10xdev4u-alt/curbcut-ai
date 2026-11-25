#!/usr/bin/env python3
"""Fail if the hash statement loses a property or a cross-reference.

CC-202 asked for one canonical characterization of every hash construction,
referenced from the documents that used to describe them differently. This
script re-reads that statement and its consumers and checks the facts that
matter:

* ``docs/HASHES.md`` keeps its five sections and the properties that make the
  characterization honest: the cryptographic family, the 64-bit identity
  family, the birthday bound, the cross-release stability caveat, the
  "not a security boundary" disclaimer, the FNV-1a correlation token, and the
  rule that no hash is applied to raw screen content. It also states its own
  scope: user data and stored state.
* ``docs/STABILITY.md``, ``docs/READING_LEVEL.md``, and ``docs/PROVENANCE.md``
  link to it and use the same vocabulary. A document that calls an identity
  hash "non-reversible" or "cryptographic" contradicts the statement.
* ``docs/PRIVACY_FINDINGS.md`` does not still carry ``F-07`` as open,
  ``docs/DPIA.md`` points ``R-08`` at the statement and no longer lists it
  among the unreconciled claims, ``README.md`` indexes both the document and
  this script, and CI runs this script.

This checks prose structure and cross-references, not the Rust code: the code
is covered by ``cargo test``. Run ``--self-test`` to apply every named
mutation to a pristine copy and require detection.
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

HASHES = "docs/HASHES.md"
CONSUMERS = {
    "docs/STABILITY.md": "stability",
    "docs/READING_LEVEL.md": "reading-level",
    "docs/PROVENANCE.md": "provenance",
}
README = "README.md"
WORKFLOW = ".github/workflows/ci.yml"
FINDINGS = "docs/PRIVACY_FINDINGS.md"
DPIA = "docs/DPIA.md"

# Source comments that characterize a hash. They must match docs/HASHES.md too.
SOURCES = {
    "crates/curbcut-a11y/src/stability.rs": (
        "identity hash",
        ("not reversible",),
    ),
    "crates/curbcut-tts/src/engine.rs": (
        "non-cryptographic",
        ("hash of the text, for telemetry",),
    ),
}

DOCUMENTS = (HASHES, *CONSUMERS, README, WORKFLOW, FINDINGS, DPIA, *SOURCES)

REQUIRED_SECTIONS = (
    "The constructions",
    "Cryptographic family: SHA-256 and HMAC-SHA-256",
    "Non-cryptographic family: 64-bit identity hashes",
    "Recovery",
    "What each hash is applied to",
)

REQUIRED_PHRASES = (
    "sha-256",
    "hmac-sha-256",
    "defaulthasher",
    "siphash-1-3",
    "2^32",
    "across rust releases",
    "not a security boundary",
    "applied to raw screen content",
    "applied to user data or stored state",
    "fnv-1a",
    "dc-31",
)

# Characterizations that contradict docs/HASHES.md. The lookbehind keeps the
# legitimate "non-cryptographic ... hash" phrasing from matching.
OVERCLAIMS = (
    re.compile(r"non-reversible hash"),
    re.compile(r"(?<!non-)cryptographic (?:content|identity) hash"),
)


def flatten(text: str) -> str:
    return " ".join(text.lower().split())


def table_cells(text: str, first_cell: str) -> list[str] | None:
    """Returns the cells of the table row whose first cell is ``first_cell``."""
    for line in text.splitlines():
        stripped = line.strip()
        if not (stripped.startswith("|") and stripped.endswith("|")):
            continue
        cells = [cell.strip() for cell in stripped.strip("|").split("|")]
        if cells and cells[0] == first_cell:
            return cells
    return None


def section(text: str, start: str, end: str) -> str:
    """Returns the text between two headings, or an empty string."""
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

    hashes = documents[HASHES]
    for heading in REQUIRED_SECTIONS:
        if not re.search(rf"^#{{1,6}}\s+{re.escape(heading)}\b", hashes, re.MULTILINE):
            errors.append(f"docs/HASHES.md is missing the {heading!r} section")
    body = flatten(hashes)
    for phrase in REQUIRED_PHRASES:
        if phrase not in body:
            errors.append(f"docs/HASHES.md no longer states {phrase!r}")

    for relative, name in CONSUMERS.items():
        text = documents[relative]
        if "](HASHES.md)" not in text:
            errors.append(f"{relative} no longer links to docs/HASHES.md")
        if "identity hash" not in flatten(text):
            errors.append(f"{relative} no longer uses the term 'identity hash'")
        if "not a security boundary" not in flatten(text):
            errors.append(f"{relative} no longer disclaims the security boundary")
        for pattern in OVERCLAIMS:
            match = pattern.search(flatten(text))
            if match:
                errors.append(
                    f"{relative} calls an identity hash {match.group(0)!r}, which "
                    "contradicts docs/HASHES.md"
                )

    for relative, (required, banned) in SOURCES.items():
        body = flatten(documents[relative])
        if required not in body:
            errors.append(f"{relative} no longer states {required!r}")
        for phrase in banned:
            if phrase in body:
                errors.append(
                    f"{relative} still claims {phrase!r}, which contradicts "
                    "docs/HASHES.md"
                )

    readme = flatten(documents[README])
    if "hashes.md" not in readme:
        errors.append("README.md no longer indexes docs/HASHES.md")
    if "validate_hashes.py" not in readme:
        errors.append("README.md no longer indexes scripts/validate_hashes.py")
    if "four hash constructions" not in readme:
        errors.append("README.md no longer states four hash constructions")
    if "three hash constructions" in readme:
        errors.append("README.md still says three hash constructions")

    workflow = documents[WORKFLOW]
    if "scripts/validate_hashes.py --self-test" not in workflow:
        errors.append("CI no longer self-tests the hash-statement validator")
    if not re.search(r"python3 scripts/validate_hashes\.py\n", workflow):
        errors.append("CI no longer runs the hash-statement validator")

    findings = documents[FINDINGS]
    row = table_cells(findings, "F-07")
    if row is None:
        errors.append("PRIVACY_FINDINGS.md has no F-07 row")
    elif len(row) < 3:
        errors.append("F-07 row is truncated")
    elif row[2] in {"OPEN", "IN PROGRESS"}:
        errors.append("F-07 still claims the hash characterization is inconsistent")

    dpia = documents[DPIA]
    risk = table_cells(dpia, "R-08")
    if risk is None:
        errors.append("DPIA.md has no R-08 row")
    elif len(risk) < 5:
        errors.append("R-08 row is truncated")
    elif "HASHES.md" not in risk[4]:
        errors.append("R-08 mitigation no longer points at docs/HASHES.md")
    residual = section(dpia, "Residual risk", "Consultation and review")
    if residual and "R-08" in residual:
        errors.append("DPIA residual risk still lists R-08 as unreconciled")
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
        "label": "canonical statement deleted",
        "delete": [HASHES],
        "expect": "missing docs/HASHES.md",
    },
    {
        "label": "constructions section deleted",
        "edit": [(HASHES, "## The constructions", "## Constructions")],
        "expect": "'The constructions' section",
    },
    {
        "label": "cryptographic family section deleted",
        "edit": [
            (HASHES, "## Cryptographic family: SHA-256 and HMAC-SHA-256", "## Family")
        ],
        "expect": "'Cryptographic family: SHA-256 and HMAC-SHA-256' section",
    },
    {
        "label": "identity family section deleted",
        "edit": [
            (
                HASHES,
                "## Non-cryptographic family: 64-bit identity hashes",
                "## Identity",
            )
        ],
        "expect": "'Non-cryptographic family: 64-bit identity hashes' section",
    },
    {
        "label": "recovery section deleted",
        "edit": [(HASHES, "## Recovery", "## Recovering")],
        "expect": "'Recovery' section",
    },
    {
        "label": "security-boundary disclaimer removed",
        "edit": [(HASHES, "not a security boundary", "not a boundary")],
        "expect": "'not a security boundary'",
    },
    {
        "label": "birthday bound removed",
        "edit": [(HASHES, "2^32 distinct values", "many distinct values")],
        "expect": "'2^32'",
    },
    {
        "label": "cross-release caveat removed",
        "edit": [(HASHES, "across Rust releases", "between builds")],
        "expect": "'across rust releases'",
    },
    {
        "label": "raw-screen-content rule removed",
        "edit": [(HASHES, "applied to raw screen content", "used in diagnostics")],
        "expect": "'applied to raw screen content'",
    },
    {
        "label": "FNV-1a token removed",
        "replace": [(HASHES, "FNV-1a", "FNV")],
        "expect": "'fnv-1a'",
    },
    {
        "label": "model-pin class removed",
        "replace": [(HASHES, "`DC-31`", "`DC-01`")],
        "expect": "'dc-31'",
    },
    {
        "label": "scope sentence removed",
        "edit": [
            (
                HASHES,
                "for every hash applied to user data or\nstored state",
                "for every construction",
            )
        ],
        "expect": "'applied to user data or stored state'",
    },
    {
        "label": "stability source comment reverts to non-reversible",
        "edit": [
            (
                "crates/curbcut-a11y/src/stability.rs",
                "This is a non-cryptographic identity hash: stable for the same input, not a",
                "The hash is stable for the same input and not reversible; it is",
            )
        ],
        "expect": "not reversible",
    },
    {
        "label": "tts source comment claims telemetry",
        "edit": [
            (
                "crates/curbcut-tts/src/engine.rs",
                "A non-cryptographic identity hash of the text, for log-safe summaries",
                "A non-cryptographic hash of the text, for telemetry",
            )
        ],
        "expect": "hash of the text, for telemetry",
    },
    {
        "label": "stability drops the link",
        "replace": [("docs/STABILITY.md", "[`docs/HASHES.md`](HASHES.md)", "")],
        "expect": "no longer links to docs/HASHES.md",
    },
    {
        "label": "reading level drops the link",
        "replace": [("docs/READING_LEVEL.md", "[`docs/HASHES.md`](HASHES.md)", "")],
        "expect": "no longer links to docs/HASHES.md",
    },
    {
        "label": "provenance drops the link",
        "replace": [("docs/PROVENANCE.md", "[`docs/HASHES.md`](HASHES.md)", "")],
        "expect": "no longer links to docs/HASHES.md",
    },
    {
        "label": "stability calls the identity hash cryptographic",
        "edit": [
            ("docs/STABILITY.md", "non-cryptographic identity hash", "cryptographic content hash")
        ],
        "expect": "cryptographic content hash",
    },
    {
        "label": "reading level calls the identity hash non-reversible",
        "edit": [
            ("docs/READING_LEVEL.md", "non-cryptographic identity hash", "non-reversible hash")
        ],
        "expect": "non-reversible hash",
    },
    {
        "label": "readme drops the script",
        "edit": [(README, "validate_hashes.py", "validate_removed.py")],
        "expect": "no longer indexes scripts/validate_hashes.py",
    },
    {
        "label": "readme hash count reverts",
        "edit": [(README, "The four hash constructions", "The three hash constructions")],
        "expect": "still says three hash constructions",
    },
    {
        "label": "CI drops the self-test",
        "edit": [(WORKFLOW, "scripts/validate_hashes.py --self-test", "scripts/check_no_content_logging.py")],
        "expect": "no longer self-tests the hash-statement validator",
    },
    {
        "label": "finding reopened",
        "edit": [(FINDINGS, "| F-07 | LOW | MITIGATED |", "| F-07 | LOW | OPEN |")],
        "expect": "F-07 still claims",
    },
    {
        "label": "risk mitigation detached",
        "edit": [(DPIA, "`docs/HASHES.md` states each construction once", "the documents agree")],
        "expect": "R-08 mitigation no longer points at docs/HASHES.md",
    },
    {
        "label": "residual risk re-lists the hash claim",
        "edit": [(DPIA, "- **Untested public claims (R-05).**", "- **Untested public claims (R-05, R-08).**")],
        "expect": "residual risk still lists R-08",
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
    for relative, old, new in case.get("replace", ()):  # type: ignore[union-attr]
        path = tmp / str(relative)
        text = path.read_text(encoding="utf-8")
        if old not in text:
            raise AssertionError(f"case no longer applies: {old!r} not in {relative}")
        path.write_text(text.replace(old, new), encoding="utf-8", newline="\n")


def self_test() -> int:
    pristine = {
        relative: (ROOT / relative).read_text(encoding="utf-8")
        for relative in collected_files(ROOT)
    }
    tmp = Path(tempfile.mkdtemp(prefix="curbcut-hashes-"))
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
        print(f"{len(errors)} hash-statement check(s) failed", file=sys.stderr)
        return 1
    print(
        "one hash statement, three cross-referenced consumers, and a closed finding"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
