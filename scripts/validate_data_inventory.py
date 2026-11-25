#!/usr/bin/env python3
"""Fail if the privacy inventory loses a basis, a retention rule, or a link.

CC-096 requires every data class to carry a lawful basis and a retention rule,
and every finding to be tracked. Both are prose tables, so a well-meaning edit
can blank a cell or point a row at an issue that does not exist. This script
re-reads the three documents and checks the facts that matter:

* ``docs/DATA_INVENTORY.md`` — every class has an ID, a retention value, a
  lawful basis, a consent gate, and a valid code status, all from closed
  vocabularies, and every finding reference resolves. References must agree in
  both directions: a class pointing at a finding whose own class list omits it
  fails. Ranges in the rationale prose are bounds-checked.
* ``docs/DPIA.md`` — keeps its required sections, states its class count,
  validates the R-xx risk table (header, sequential IDs, likelihood and impact
  vocabularies, resolvable finding links), and keeps the disclaimer that it is
  internal work rather than legal advice.
* ``docs/PRIVACY_FINDINGS.md`` — every finding has a severity, a status, a
  class reference, an owner, a due date, and, while open, somewhere to track it.

This checks structure, vocabularies, and cross-references. It cannot tell
whether a retention value that is still *in* the vocabulary is the right one,
or whether a consent gate describes what the code actually enforces; those need
a reader.

Usage::

    python3 scripts/validate_data_inventory.py            # validate the repo
    python3 scripts/validate_data_inventory.py --self-test  # prove it fails

``--self-test`` copies the documents into a temporary tree, applies one named
deletion per guarantee, and requires each to be detected. It fails if a
document is missing. Values outside a vocabulary fail the ordinary run, which
is what stops a retention rule from quietly becoming a word nobody implements.
"""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
INVENTORY = ROOT / "docs" / "DATA_INVENTORY.md"
DPIA = ROOT / "docs" / "DPIA.md"
FINDINGS = ROOT / "docs" / "PRIVACY_FINDINGS.md"

INVENTORY_COLUMNS = [
    "ID",
    "Class",
    "Source",
    "Stored",
    "Retention",
    "Lawful basis",
    "Consent gate",
    "Leaves device",
    "Sensitive",
    "Code status",
    "Finding",
]

FINDING_COLUMNS = [
    "ID",
    "Severity",
    "Status",
    "Classes",
    "Summary",
    "Owner",
    "Due",
    "Tracking",
]

RISK_COLUMNS = ["ID", "Risk", "Likelihood", "Impact", "Mitigation", "Finding"]

LIKELIHOOD = {"low", "medium", "high", "certain"}
IMPACT = {"severe", "high", "medium", "low"}

# The DPIA's published claim is ten named risks. Deriving the expectation from
# whatever rows survive let a deleted row pass, so the count is fixed here.
RISK_COUNT = 10

RETENTION = {
    "2-seconds",
    "<1-second",
    "frame-to-frame",
    "session",
    "bounded-lru",
    "bounded-capacity",
    "bounded-records",
    "7-days",
    "30-days",
    "90-days",
    "18-months",
    "until-deleted",
    "never-stored",
    "disputed",
    "unimplemented",
}

BASIS = {
    "consent",
    "contract",
    "legitimate-interests",
    "legal-obligation",
    "none-transient",
    "undetermined",
}

LEAVES = {"local-only", "opt-in", "always", "never", "user-export"}
SENSITIVE = {"yes", "derived", "hash-only", "no"}
CODE_STATUS = {"in-memory", "implemented", "planned", "boundary-missing"}
SEVERITY = {"CRITICAL", "HIGH", "MEDIUM", "LOW"}
STATUS = {"OPEN", "IN PROGRESS", "MITIGATED", "ACCEPTED", "WONTFIX"}

DPIA_SECTIONS = (
    "Scope",
    "Inventory coverage",
    "Lawful bases",
    "Necessity and proportionality",
    "Risks and mitigations",
    "Residual risk",
    "Consultation and review",
)

DUE_RE = re.compile(r"\d{4}-\d{2}-\d{2}")
CLASS_RE = re.compile(r"DC-\d+")
FINDING_RE = re.compile(r"F-\d+")


def cells(line: str) -> list[str]:
    """Split a markdown table row into trimmed cells."""
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def is_separator(line: str) -> bool:
    return bool(cells(line)) and all(
        re.fullmatch(r":?-{2,}:?", cell) for cell in cells(line)
    )


def table_rows(text: str) -> list[str]:
    """Every markdown table row, separator included."""
    return [line.strip() for line in text.splitlines() if line.strip().startswith("|")]


def data_rows(text: str, prefix: str) -> list[tuple[str, list[str]]]:
    """Rows whose first cell starts with ``prefix``, paired with their line."""
    rows: list[tuple[str, list[str]]] = []
    for line in table_rows(text):
        if is_separator(line):
            continue
        row = cells(line)
        if row and row[0].startswith(prefix):
            rows.append((line, row))
    return rows


def header_rows(text: str) -> list[list[str]]:
    """Header rows: a table row immediately followed by a separator."""
    lines = [line.strip() for line in text.splitlines()]
    headers: list[list[str]] = []
    for index, line in enumerate(lines):
        if not line.startswith("|") or is_separator(line):
            continue
        following = lines[index + 1].strip() if index + 1 < len(lines) else ""
        if following.startswith("|") and is_separator(following):
            headers.append(cells(line))
    return headers


def tokens(cell: str) -> list[str]:
    return [token.strip() for token in cell.split(",") if token.strip()]


def plain(cell: str) -> str:
    """Strip emphasis so `` `<1-second` `` matches the vocabulary entry."""
    return cell.replace("`", "").replace("*", "").strip()


CLASS_RANGE_RE = re.compile(r"DC-(\d+)\s+through\s+DC-(\d+)")


def expand_classes(cell: str) -> list[str]:
    """Expand a class cell such as ``DC-01 through DC-03, DC-07`` into IDs.

    Malformed entries are returned as their raw text so the caller can report
    them rather than silently skipping the check.
    """
    expanded: list[str] = []
    for token in tokens(cell):
        if token == "-":
            continue
        span = CLASS_RANGE_RE.fullmatch(token)
        if span:
            first, last = int(span.group(1)), int(span.group(2))
            expanded.extend(f"DC-{index:02d}" for index in range(first, last + 1))
        else:
            expanded.append(token)
    return expanded


def validate_inventory(text: str) -> tuple[list[str], list[str]]:
    """Return (errors, class ids in document order)."""
    errors: list[str] = []

    headers = header_rows(text)
    class_headers = [header for header in headers if header == INVENTORY_COLUMNS]
    # The inventory also carries a lawful-basis rationale table with its own
    # columns, so require the class tables specifically rather than every header.
    if len(class_headers) != 4:
        errors.append(
            f"inventory needs four class tables with the exact header "
            f"{', '.join(INVENTORY_COLUMNS)}; found {len(class_headers)}"
        )
    if not headers:
        errors.append("inventory has no table with a header and separator")

    rows = data_rows(text, "DC-")
    if not rows:
        errors.append("inventory has no data classes")

    ids: list[str] = []
    for _, row in rows:
        if len(row) != len(INVENTORY_COLUMNS):
            errors.append(
                f"inventory row {row[0] if row else '?'} has {len(row)} cells, "
                f"expected {len(INVENTORY_COLUMNS)}"
            )
            continue
        class_id, name, source, stored, retention, basis, gate, leaves, sensitive, code, finding = row
        ids.append(class_id)
        if not name or not source or not stored:
            errors.append(f"{class_id} has an empty class, source, or stored cell")
        retention = plain(retention)
        basis = plain(basis)
        leaves = plain(leaves)
        sensitive = plain(sensitive)
        code = plain(code)
        if retention not in RETENTION:
            errors.append(
                f"{class_id} retention {retention!r} is not in the vocabulary"
            )
        if basis not in BASIS:
            errors.append(f"{class_id} lawful basis {basis!r} is not in the vocabulary")
        if not gate:
            errors.append(f"{class_id} has no consent gate")
        if leaves not in LEAVES:
            errors.append(f"{class_id} leaves-device {leaves!r} is not in the vocabulary")
        if sensitive not in SENSITIVE:
            errors.append(f"{class_id} sensitivity {sensitive!r} is not in the vocabulary")
        if code not in CODE_STATUS:
            errors.append(f"{class_id} code status {code!r} is not in the vocabulary")
        for token in tokens(finding):
            if token == "-":
                continue
            if not FINDING_RE.fullmatch(token):
                errors.append(f"{class_id} finding reference {token!r} is malformed")
            elif token not in KNOWN_FINDINGS:
                errors.append(f"{class_id} points at unknown finding {token}")

    expected = [f"DC-{index:02d}" for index in range(1, len(rows) + 1)]
    if ids != expected:
        errors.append(
            "inventory IDs must be sequential from DC-01 with no gaps or duplicates; "
            f"found {ids[:3]}...{ids[-3:] if len(ids) > 3 else ''}"
        )

    # Ranges also appear in the lawful-basis rationale prose, where a typo would
    # silently claim coverage of classes that do not exist.
    for span in CLASS_RANGE_RE.finditer(text):
        first, last = int(span.group(1)), int(span.group(2))
        if last > len(rows):
            errors.append(
                f"range {span.group(0)!r} reaches DC-{last:02d} but the inventory "
                f"has {len(rows)} classes"
            )
        elif first > last:
            errors.append(f"range {span.group(0)!r} is reversed")
    return errors, ids


def validate_findings(text: str, class_ids: list[str]) -> list[str]:
    errors: list[str] = []
    headers = header_rows(text)
    if not headers:
        errors.append("findings register has no table with a header and separator")
    for header in headers:
        if header != FINDING_COLUMNS:
            errors.append(
                "findings header must be exactly: " + ", ".join(FINDING_COLUMNS)
            )

    rows = data_rows(text, "F-")
    if not rows:
        errors.append("findings register has no findings")

    ids: list[str] = []
    for _, row in rows:
        if len(row) != len(FINDING_COLUMNS):
            errors.append(
                f"finding row {row[0] if row else '?'} has {len(row)} cells, "
                f"expected {len(FINDING_COLUMNS)}"
            )
            continue
        finding_id, severity, status, classes, summary, owner, due, tracking = row
        ids.append(finding_id)
        if severity not in SEVERITY:
            errors.append(f"{finding_id} severity {severity!r} is not in the vocabulary")
        if status not in STATUS:
            errors.append(f"{finding_id} status {status!r} is not in the vocabulary")
        if not summary or not owner:
            errors.append(f"{finding_id} has an empty summary or owner")
        if not DUE_RE.fullmatch(due):
            errors.append(f"{finding_id} due date {due!r} is not YYYY-MM-DD")
        for class_id in expand_classes(classes):
            if not CLASS_RE.fullmatch(class_id):
                errors.append(f"{finding_id} class reference {class_id!r} is malformed")
            elif class_id not in class_ids:
                errors.append(f"{finding_id} points at unknown class {class_id}")
        if tracking != "-" and not re.fullmatch(r"#\d+", tracking):
            errors.append(f"{finding_id} tracking {tracking!r} is not '#<issue>' or '-'")
        if status in {"OPEN", "IN PROGRESS"} and tracking == "-":
            errors.append(f"{finding_id} is {status} but has nowhere to track the work")

    expected = [f"F-{index:02d}" for index in range(1, len(rows) + 1)]
    if ids != expected:
        errors.append("finding IDs must be sequential from F-01 with no gaps or duplicates")
    return errors


def validate_risks(text: str, known_findings: set[str]) -> list[str]:
    """Check the R-xx register: it is a claim that exists only in this table."""
    errors: list[str] = []
    risk_headers = [header for header in header_rows(text) if header == RISK_COLUMNS]
    if len(risk_headers) != 1:
        errors.append(
            "DPIA needs exactly one risks table with the header "
            + ", ".join(RISK_COLUMNS)
        )

    rows = data_rows(text, "R-")
    ids: list[str] = []
    for _, row in rows:
        if len(row) != len(RISK_COLUMNS):
            errors.append(
                f"risk row {row[0] if row else '?'} has {len(row)} cells, "
                f"expected {len(RISK_COLUMNS)}"
            )
            continue
        risk_id, risk, likelihood, impact, mitigation, finding = row
        ids.append(risk_id)
        if not risk or not mitigation:
            errors.append(f"{risk_id} has an empty risk or mitigation cell")
        if plain(likelihood).lower() not in LIKELIHOOD:
            errors.append(f"{risk_id} likelihood {likelihood!r} is not in the vocabulary")
        if plain(impact).lower() not in IMPACT:
            errors.append(f"{risk_id} impact {impact!r} is not in the vocabulary")
        for token in tokens(finding):
            if token == "-":
                continue
            if not FINDING_RE.fullmatch(token):
                errors.append(f"{risk_id} finding reference {token!r} is malformed")
            elif token not in known_findings:
                errors.append(f"{risk_id} points at unknown finding {token}")

    expected = [f"R-{index:02d}" for index in range(1, RISK_COUNT + 1)]
    if ids != expected:
        errors.append(
            f"risk IDs must be exactly R-01 through R-{RISK_COUNT:02d}; "
            "a deleted row cannot shrink the register"
        )
    return errors


def validate_dpia(text: str, class_count: int, known_findings: set[str]) -> list[str]:
    errors: list[str] = []
    lowered = text.lower()
    for section in DPIA_SECTIONS:
        # Require a heading, not a mention: prose inside the assessment quotes
        # these phrases, so a substring test survived deleting the heading.
        pattern = rf"^#{{1,6}}\s+{re.escape(section)}\b"
        if not re.search(pattern, text, re.IGNORECASE | re.MULTILINE):
            errors.append(f"DPIA is missing the {section} section")

    match = re.search(r"covers\s+(\d+)\s+data\s+classes", lowered)
    if match is None:
        errors.append("DPIA does not state how many data classes the inventory covers")
    elif int(match.group(1)) != class_count:
        errors.append(
            f"DPIA claims {match.group(1)} classes but the inventory has {class_count}"
        )

    if "not legal advice" not in lowered:
        errors.append("DPIA no longer states that it is not legal advice")
    if "external privacy auditor" not in lowered:
        errors.append("DPIA no longer states that the external audit is outstanding")
    if "does not assert" not in lowered:
        errors.append("DPIA no longer disclaims compliance")
    errors.extend(validate_risks(text, known_findings))
    return errors


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    for path in (INVENTORY, DPIA, FINDINGS):
        if not (root / path.relative_to(ROOT)).exists():
            errors.append(f"missing document: {path.relative_to(ROOT)}")
    if errors:
        return errors

    global KNOWN_FINDINGS
    findings_text = (root / FINDINGS.relative_to(ROOT)).read_text(encoding="utf-8")
    KNOWN_FINDINGS = {row[1][0] for row in data_rows(findings_text, "F-")}
    if not KNOWN_FINDINGS:
        errors.append("findings register has no findings")

    inventory_text = (root / INVENTORY.relative_to(ROOT)).read_text(encoding="utf-8")
    inventory_errors, class_ids = validate_inventory(inventory_text)
    errors.extend(f"{INVENTORY.relative_to(ROOT)}: {message}" for message in inventory_errors)
    errors.extend(
        f"{FINDINGS.relative_to(ROOT)}: {message}"
        for message in validate_findings(findings_text, class_ids)
    )

    # Both sides of a reference must agree. Checking only that the finding
    # exists let DC-30 point at F-08 while F-12 described DC-30, and let
    # DC-01..03 point at a finding whose class list omitted them.
    finding_classes: dict[str, set[str]] = {}
    for _, row in data_rows(findings_text, "F-"):
        if len(row) == len(FINDING_COLUMNS):
            finding_classes[row[0]] = set(expand_classes(row[3]))
    for _, row in data_rows(inventory_text, "DC-"):
        if len(row) != len(INVENTORY_COLUMNS):
            continue
        class_id = row[0]
        for token in tokens(row[10]):
            if token == "-" or token not in finding_classes:
                continue
            if class_id not in finding_classes[token]:
                errors.append(
                    f"{INVENTORY.relative_to(ROOT)}: {class_id} points at "
                    f"{token}, but that finding does not list {class_id}"
                )

    dpia_text = (root / DPIA.relative_to(ROOT)).read_text(encoding="utf-8")
    errors.extend(
        f"{DPIA.relative_to(ROOT)}: {message}"
        for message in validate_dpia(dpia_text, len(class_ids), KNOWN_FINDINGS)
    )
    return errors


KNOWN_FINDINGS: set[str] = set()


def _mutated_copy(root: Path) -> Path:
    tmp = Path(tempfile.mkdtemp(prefix="curbcut-privacy-selftest-"))
    (tmp / "docs").mkdir()
    for source in (INVENTORY, DPIA, FINDINGS):
        target = tmp / source.relative_to(ROOT)
        target.write_text(source.read_text(encoding="utf-8"), encoding="utf-8")
    return tmp


def _break(document: Path, old: str, new: str) -> None:
    text = document.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"self-test fixture no longer matches {document.name}: {old!r}")
    document.write_text(text.replace(old, new, 1), encoding="utf-8")


MUTATIONS: tuple[tuple[str, str, str, str], ...] = (
    ("retention vocabulary", "docs/DATA_INVENTORY.md", "| 2-seconds | contract |", "| forever | contract |"),
    ("lawful basis vocabulary", "docs/DATA_INVENTORY.md", "| until-deleted | consent | none required to author", "| until-deleted | because | none required to author"),
    ("consent gate", "docs/DATA_INVENTORY.md", "| contract | screen-recording permission, stopped by the panic key |", "| contract |  |"),
    ("leaves-device vocabulary", "docs/DATA_INVENTORY.md", "| local-only | yes | in-memory | F-05 |", "| local-only | sometimes | in-memory | F-05 |"),
    ("sensitivity vocabulary", "docs/DATA_INVENTORY.md", "| opt-in | derived | planned | F-01, F-09 |", "| opt-in | very | planned | F-01, F-09 |"),
    ("code-status vocabulary", "docs/DATA_INVENTORY.md", "| derived | planned | F-01, F-09 |", "| derived | someday | F-01, F-09 |"),
    ("duplicate class id", "docs/DATA_INVENTORY.md", "| DC-02 | OCR text", "| DC-01 | OCR text"),
    (
        "dangling finding",
        "docs/DATA_INVENTORY.md",
        "| local-only | yes | in-memory | F-05 |",
        "| local-only | yes | in-memory | F-99 |",
    ),
    ("malformed row", "docs/DATA_INVENTORY.md", "| frame sampling | RAM, 32 bytes | frame-to-frame |", "| frame sampling | frame-to-frame |"),
    ("dpia class count", "docs/DPIA.md", "covers 37 data classes", "covers 36 data classes"),
    ("dpia section", "docs/DPIA.md", "## Residual risk", "## What is left"),
    ("dpia disclaimer", "docs/DPIA.md", "Not legal advice.", "Legal advice."),
    ("finding severity", "docs/PRIVACY_FINDINGS.md", "| F-03 | MEDIUM | OPEN |", "| F-03 | SERIOUS | OPEN |"),
    ("finding status", "docs/PRIVACY_FINDINGS.md", "| F-07 | LOW | MITIGATED |", "| F-07 | LOW | MAYBE |"),
    ("finding due date", "docs/PRIVACY_FINDINGS.md", "| Founding engineer | 2027-03-15 | #202 |", "| Founding engineer | soon | #202 |"),
    ("finding class reference", "docs/PRIVACY_FINDINGS.md", "| DC-05, DC-07, DC-20 |", "| DC-05, DC-07, DC-99 |"),
    ("open finding untracked", "docs/PRIVACY_FINDINGS.md", "| Security lead | 2027-01-31 | #198 |", "| Security lead | 2027-01-31 | - |"),
    ("finding id sequence", "docs/PRIVACY_FINDINGS.md", "| F-08 | HIGH |", "| F-80 | HIGH |"),
    ("register header", "docs/PRIVACY_FINDINGS.md", "| ID | Severity | Status | Classes | Summary | Owner | Due | Tracking |\n", ""),
    (
        "class-finding round trip",
        "docs/DATA_INVENTORY.md",
        "| RAM today; disk persistence planned | session | legitimate-interests | kill switch denies the channel | always | no | in-memory | F-12 |",
        "| RAM today; disk persistence planned | session | legitimate-interests | kill switch denies the channel | always | no | in-memory | F-08 |",
    ),
    (
        "prose class range",
        "docs/DATA_INVENTORY.md",
        "DC-01 through DC-06, DC-08",
        "DC-01 through DC-99, DC-08",
    ),
    (
        "risk likelihood vocabulary",
        "docs/DPIA.md",
        "| Medium | High | Explicit consent gates",
        "| Sometimes | High | Explicit consent gates",
    ),
    (
        "risk finding link",
        "docs/DPIA.md",
        "Deletion controls exist for some classes; no cadence | F-10 |",
        "Deletion controls exist for some classes; no cadence | F-99 |",
    ),
    (
        "risk row deleted",
        "docs/DPIA.md",
        "| R-10 | Retained data outlives the reason it was kept | Medium | Medium | Deletion controls exist for some classes; no cadence | F-10 |\n",
        "",
    ),
    (
        "risk header",
        "docs/DPIA.md",
        "| ID | Risk | Likelihood | Impact | Mitigation | Finding |\n",
        "",
    ),
)


def self_test() -> int:
    root = _mutated_copy(ROOT)
    try:
        if validate(root):
            print("SELF-TEST FAILED: a pristine copy did not pass", file=sys.stderr)
            return 1
        print("self-test: the committed documents pass")

        # Restore the exact bytes between cases. Reverting with str.replace is
        # not enough: for a deletion (`new == ""`) it would insert the original
        # at byte 0, leaving later mutations to fail on the leftover corruption
        # rather than on the change under test.
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

        (root / FINDINGS.relative_to(ROOT)).unlink()
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
        print(f"{len(errors)} privacy-inventory check(s) failed", file=sys.stderr)
        return 1
    print("every data class has a basis, a retention rule, and a resolved finding link")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
