#!/usr/bin/env python3
"""Render the accessibility documents from the mapping data.

``WCAG``, ``COGA``, and ``LIMITATIONS`` in
``crates/curbcut-cli/src/vpat.rs`` are the single source of truth. This script
extracts them and writes three documents, so a claim and the test that backs it
cannot drift apart:

* ``docs/A11Y_MAPPING.md`` — the WCAG and COGA tables.
* ``docs/VPAT.md`` — the draft VPAT for CurbCut's own UI.
* ``docs/LIMITATIONS.md`` — the published limitations.

Usage:

    python3 scripts/render_a11y_mapping.py           # write the documents
    python3 scripts/render_a11y_mapping.py --check   # fail if a doc is stale
    python3 scripts/render_a11y_mapping.py --self-test

``--check`` also resolves every row's test link, so a renamed test fails the
build rather than quietly orphaning a claim.

Exit codes:
    0  the documents are current (or were written), or the self-test passed
    1  a document is stale, a link does not resolve, or the self-test failed
    2  the mapping source could not be parsed
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "crates" / "curbcut-cli" / "src" / "vpat.rs"
TARGETS = {
    "mapping": ROOT / "docs" / "A11Y_MAPPING.md",
    "vpat": ROOT / "docs" / "VPAT.md",
    "limitations": ROOT / "docs" / "LIMITATIONS.md",
}
AUDIT_DOC = "docs/A11Y_AUDIT.md"

LEVELS = {"A": "A", "Aa": "AA", "Aaa": "AAA"}
STATUS_LABELS = {
    "Supports": "Supports",
    "Partial": "Partially supports",
    "NotApplicable": "Not applicable",
    "NotAssessed": "Not assessed",
}
STATUS_VPAT = {
    "Supports": "Supports",
    "Partial": "Partially Supports",
    "NotApplicable": "Not Applicable",
    "NotAssessed": "Not Assessed",
}
KINDS = {"Automated": "automated", "Manual": "manual"}

WCAG_BLOCK_RE = re.compile(
    r"pub const WCAG: &\[MappingRow\] = &\[(?P<body>.*?)\n\];", re.DOTALL
)
COGA_BLOCK_RE = re.compile(
    r"pub const COGA: &\[CogaRow\] = &\[(?P<body>.*?)\n\];", re.DOTALL
)
ROW_RE = re.compile(r"MappingRow\s*\{(?P<body>.*?)\n\s*\},", re.DOTALL)
COGA_RE = re.compile(r"CogaRow\s*\{(?P<body>.*?)\n\s*\},", re.DOTALL)
LIMIT_RE = re.compile(r"pub const LIMITATIONS: &\[&str\] = &\[(?P<body>.*?)\n\];", re.DOTALL)
STRING_RE = r'"((?:[^"\\]|\\.)*)"'


class SourceError(RuntimeError):
    """The mapping source could not be parsed."""


def _unescape(text: str) -> str:
    """Decodes the two escapes the mapping data uses."""
    return text.replace(r"\"", '"').replace(r"\\", "\\")


def _string_field(body: str, name: str, where: str) -> str:
    match = re.search(rf"{name}:\s*{STRING_RE}", body)
    if match is None:
        raise SourceError(f"{where}: field {name} is missing or not a string")
    return _unescape(match.group(1))


def _enum_field(body: str, name: str, enum: str, table: dict[str, str], where: str) -> str:
    match = re.search(rf"{name}:\s*{enum}::(\w+)", body)
    if match is None:
        raise SourceError(f"{where}: field {name} is missing")
    variant = match.group(1)
    if variant not in table:
        raise SourceError(f"{where}: unknown {enum}::{variant}")
    return variant


def parse_rows(source: str) -> list[dict[str, str]]:
    """Extracts the WCAG rows from the Rust source."""
    block = WCAG_BLOCK_RE.search(source)
    if block is None:
        raise SourceError("the WCAG const was not found")
    rows = []
    for match in ROW_RE.finditer(block.group("body")):
        body = match.group("body")
        where = "a MappingRow"
        criterion = _string_field(body, "criterion", where)
        rows.append(
            {
                "criterion": criterion,
                "level": LEVELS[
                    _enum_field(body, "level", "Level", LEVELS, where)
                ],
                "behavior": _string_field(body, "behavior", where),
                "status": _enum_field(body, "status", "Status", STATUS_LABELS, where),
                "note": _string_field(body, "note", where),
                "test": _string_field(body, "test", where),
                "kind": KINDS[_enum_field(body, "kind", "TestKind", KINDS, where)],
            }
        )
    if not rows:
        raise SourceError("no MappingRow found")
    return rows


def parse_coga(source: str) -> list[dict[str, str]]:
    """Extracts the COGA rows from the Rust source."""
    block = COGA_BLOCK_RE.search(source)
    if block is None:
        raise SourceError("the COGA const was not found")
    rows = []
    for match in COGA_RE.finditer(block.group("body")):
        body = match.group("body")
        where = "a CogaRow"
        rows.append(
            {
                "objective": _string_field(body, "objective", where),
                "feature": _string_field(body, "feature", where),
                "test": _string_field(body, "test", where),
                "kind": KINDS[_enum_field(body, "kind", "TestKind", KINDS, where)],
            }
        )
    if not rows:
        raise SourceError("no CogaRow found")
    return rows


def parse_limitations(source: str) -> list[str]:
    """Extracts the limitations list from the Rust source."""
    match = LIMIT_RE.search(source)
    if match is None:
        raise SourceError("LIMITATIONS not found")
    items = [_unescape(item) for item in re.findall(STRING_RE, match.group("body"))]
    if not items:
        raise SourceError("LIMITATIONS is empty")
    return items


def status_cell(row: dict[str, str]) -> str:
    """The status cell: the label plus the qualifier after a semicolon."""
    label = STATUS_LABELS[row["status"]]
    if row["note"]:
        return f"{label}; {row['note']}"
    return label


def doc_path(test: str) -> str:
    """Rewrites a repository-relative target as a ``docs/``-relative link."""
    if "#" in test:
        path, fragment = test.split("#", 1)
        return f"{_doc_base(path)}#{fragment}"
    return _doc_base(test)


def _doc_base(path: str) -> str:
    if path.startswith("docs/"):
        return path[len("docs/") :]
    return f"../{path}"


def test_cell(row: dict[str, str]) -> str:
    """The test cell: a link relative to ``docs/``."""
    return f"[{row['kind']}: `{row['test']}`]({doc_path(row['test'])})"


def capitalize(text: str) -> str:
    """Upper-cases the first character."""
    return text[:1].upper() + text[1:] if text else text


def remark(row: dict[str, str]) -> str:
    """The VPAT remark: behavior, qualifier as a sentence, and the test."""
    text = row["behavior"].rstrip(".")
    if row["note"]:
        text += ". " + capitalize(row["note"])
    if not text.endswith("."):
        text += "."
    return f"{text} Verified by {row['kind']} test: {row['test']}."


def render_wcag_table(rows: list[dict[str, str]]) -> str:
    """Renders the WCAG mapping table."""
    lines = [
        "| Criterion | Level | CurbCut behavior | Status | Test |",
        "| --- | --- | --- | --- | --- |",
    ]
    for row in rows:
        lines.append(
            f"| {row['criterion']} | {row['level']} | {row['behavior']} | "
            f"{status_cell(row)} | {test_cell(row)} |"
        )
    return "\n".join(lines) + "\n"


def render_coga_table(rows: list[dict[str, str]]) -> str:
    """Renders the COGA mapping table."""
    lines = ["| COGA objective | CurbCut feature | Test |", "| --- | --- | --- |"]
    for row in rows:
        lines.append(
            f"| {row['objective']} | {row['feature']} | {test_cell(row)} |"
        )
    return "\n".join(lines) + "\n"


def render_limitations(items: list[str]) -> str:
    """Renders the numbered limitations list."""
    return "\n".join(f"{index + 1}. {item}" for index, item in enumerate(items)) + "\n"


def render_vpat_table(rows: list[dict[str, str]]) -> str:
    """Renders the draft VPAT table and its summary counts."""
    lines = [
        "| Criterion | Level | Result | Remarks |",
        "| --- | --- | --- | --- |",
    ]
    for row in rows:
        lines.append(
            f"| {row['criterion']} | {row['level']} | "
            f"{STATUS_VPAT[row['status']]} | {remark(row)} |"
        )
    summary = (
        f"**Draft summary:** {count(rows, 'Supports')} Supports, "
        f"{count(rows, 'Partial')} Partially Supports, "
        f"{count(rows, 'NotApplicable')} Not Applicable, "
        f"{count(rows, 'NotAssessed')} Not Assessed, "
        f"across {len(rows)} criteria.\n"
    )
    return "\n".join(lines) + "\n\n" + summary


def count(rows: list[dict[str, str]], status: str) -> int:
    """Counts the rows with a status."""
    return sum(1 for row in rows if row["status"] == status)


def render_mapping(rows: list[dict[str, str]], coga: list[dict[str, str]]) -> str:
    """Renders ``docs/A11Y_MAPPING.md``."""
    return f"""# Accessibility Mapping

> What WCAG 2.2 and W3C COGA ask for, what CurbCut does about it, and what we honestly cannot do. This is the source for the VPAT and the public limitations page.

---

## 1. Framing

CurbCut is an **alternative perceptual channel**, not a conformance fix. It does not change the app's markup, semantics, or code. It can therefore help a user complete tasks in an app that is not accessible, but it cannot make that app conform to WCAG. We say this everywhere.

Two distinct claims:

- **Usability claim:** for a named persona, on a fixed benchmark, task completion and time improve by X. This is our claim.
- **Conformance claim:** the app conforms to WCAG. This is **never** our claim.

---

## 2. WCAG 2.2 mapping

{render_wcag_table(rows)}
**Honest summary:** our own UI targets WCAG 2.2 AA. The overlay's effect on the underlying app is an accommodation, not conformance.

Every row names the test that backs it. A row marked `manual` is backed by a
procedure in [`A11Y_AUDIT.md`](A11Y_AUDIT.md), not by an automated test; a row
marked `automated` names a test that runs in CI.

---

## 3. COGA mapping

{render_coga_table(coga)}
---

## 4. Known limitations (published)

The limitations page is generated from the same data. See
[`LIMITATIONS.md`](LIMITATIONS.md).

---

## 5. VPAT

The VPAT generator (`curbcut-cli vpat`) uses this table plus the automated
audit results to produce a draft. A human reviews it before publication. We
never auto-publish a "conformant" row for any third-party app. See
[`VPAT.md`](VPAT.md).
"""


def render_vpat(rows: list[dict[str, str]]) -> str:
    """Renders ``docs/VPAT.md``."""
    return f"""# Draft VPAT: CurbCut AI

> **DRAFT.** Generated from `crates/curbcut-cli/src/vpat.rs`. This VPAT
> evaluates CurbCut's own UI only. It is not a conformance claim about any
> third-party application, and it has not been reviewed by a human.

## Scope

This draft evaluates **CurbCut's own UI**: the overlay, the settings windows,
and the tray. It makes no claim about any application CurbCut is pointed at.
CurbCut is an alternative perceptual channel, not a conformance fix, so a
`Supports` here means "our UI meets the criterion", never "the app underneath
conforms".

The criteria, levels, and results come from the mapping table in
[`A11Y_MAPPING.md`](A11Y_MAPPING.md). A `Partially Supports` remark names the
limit and the test that backs the claim.

## WCAG 2.2 criteria

{render_vpat_table(rows)}
## Human review

- [ ] Every row read by a human who has seen the product.
- [ ] Every `Partially Supports` remark understood and accepted.
- [ ] Every manual test in [`A11Y_AUDIT.md`](A11Y_AUDIT.md) executed and its
      result recorded.
- [ ] The scope statement agreed: CurbCut's own UI, not a third-party app.
- [ ] The draft banner removed only after the review is recorded.

A VPAT is not published until every box is ticked and the review is recorded in
the pull request that publishes it.
"""


def render_limitations_doc(items: list[str]) -> str:
    """Renders ``docs/LIMITATIONS.md``."""
    return f"""# Limitations

> What CurbCut honestly cannot do. This page is generated from the same data as
> the accessibility mapping and the draft VPAT, so a limitation cannot be
> dropped from one place and left in another. `CC-097`.

## Published limitations

{render_limitations(items)}
## How to read these

Each limitation is a scope statement, not a defect. A limitation is changed by
building the feature, not by editing this page: edit
`crates/curbcut-cli/src/vpat.rs`, then run
`python3 scripts/render_a11y_mapping.py`.

A limitation that is fixed moves to the release notes and is removed here. A
limitation that grows becomes a `Partially Supports` row in
[`VPAT.md`](VPAT.md).
"""


def slug(text: str) -> str:
    """The GitHub-style anchor for a heading."""
    out: list[str] = []
    last_hyphen = False
    for char in text.lower():
        if char.isalnum():
            out.append(char)
            last_hyphen = False
        elif char in " -_" and not last_hyphen:
            out.append("-")
            last_hyphen = True
    return "".join(out).strip("-")


def resolve_target(test: str, root: Path = ROOT) -> tuple[bool, str]:
    """Resolves a row's test target.

    Returns `(ok, detail)`. A target is a repository-relative path with an
    optional ``#fragment``. For a Rust file the fragment must name a test; for a
    Markdown file it must be the anchor of a heading.
    """
    if "#" in test:
        path, fragment = test.split("#", 1)
    else:
        return False, f"{test}: no #fragment"
    full = root / path
    if not full.is_file():
        return False, f"{test}: {path} does not exist"
    text = full.read_text(encoding="utf-8")
    if path.endswith(".md"):
        anchors = {
            slug(line.lstrip("#").strip())
            for line in text.splitlines()
            if line.startswith("#")
        }
        if fragment not in anchors:
            return False, f"{test}: no heading with anchor {fragment}"
        return True, test
    if fragment not in text:
        return False, f"{test}: {path} does not name `{fragment}`"
    return True, test


def verify_links(
    rows: list[dict[str, str]], coga: list[dict[str, str]], root: Path = ROOT
) -> list[str]:
    """Returns a list of broken test links."""
    broken = []
    for row in rows + coga:
        ok, detail = resolve_target(row["test"], root)
        if not ok:
            broken.append(detail)
    return broken


def load() -> tuple[list[dict[str, str]], list[dict[str, str]], list[str]]:
    """Parses the mapping source."""
    source = SOURCE.read_text(encoding="utf-8")
    return parse_rows(source), parse_coga(source), parse_limitations(source)


def render_all() -> dict[str, str]:
    """Renders all three documents."""
    rows, coga, limitations = load()
    return {
        "mapping": render_mapping(rows, coga),
        "vpat": render_vpat(rows),
        "limitations": render_limitations_doc(limitations),
    }


def self_test() -> int:
    """Exercises the parser, the renderer, and the link checker."""
    synthetic = r'''
pub const WCAG: &[MappingRow] = &[
    MappingRow {
        criterion: "9.9.9 Test Criterion",
        level: Level::Aa,
        behavior: "A behavior with a \"quote\".",
        status: Status::Partial,
        note: "a qualifier",
        test: "crates/example/tests.rs#the_test",
        kind: TestKind::Automated,
    },
];
pub const COGA: &[CogaRow] = &[
    CogaRow {
        objective: "Do the thing",
        feature: "The feature",
        test: "docs/A11Y_AUDIT.md#999-test-criterion",
        kind: TestKind::Manual,
    },
];
pub const LIMITATIONS: &[&str] = &[
    "First limitation.",
    "Second limitation.",
];
'''
    rows = parse_rows(synthetic)
    assert len(rows) == 1, rows
    row = rows[0]
    assert row["criterion"] == "9.9.9 Test Criterion", row
    assert row["level"] == "AA", row
    assert row["behavior"] == 'A behavior with a "quote".', row
    assert row["status"] == "Partial", row
    assert row["note"] == "a qualifier", row
    assert row["test"] == "crates/example/tests.rs#the_test", row
    assert row["kind"] == "automated", row
    assert status_cell(row) == "Partially supports; a qualifier", status_cell(row)
    assert remark(row).endswith("crates/example/tests.rs#the_test."), remark(row)

    # A Rust `\\` escape decodes to one backslash, not two.
    escaped = parse_rows(
        r"""
pub const WCAG: &[MappingRow] = &[
    MappingRow {
        criterion: "9.9.8 Backslash",
        level: Level::A,
        behavior: "A \\backslash\\ in a path.",
        status: Status::Supports,
        note: "",
        test: "crates/example/tests.rs#the_test",
        kind: TestKind::Automated,
    },
];
"""
    )
    assert escaped[0]["behavior"] == "A \\backslash\\ in a path.", escaped[0]

    coga = parse_coga(synthetic)
    assert len(coga) == 1, coga
    assert coga[0]["objective"] == "Do the thing", coga
    assert coga[0]["kind"] == "manual", coga

    limitations = parse_limitations(synthetic)
    assert limitations == ["First limitation.", "Second limitation."], limitations
    assert render_limitations(limitations) == "1. First limitation.\n2. Second limitation.\n"

    assert doc_path("crates/x.rs#a") == "../crates/x.rs#a", doc_path("crates/x.rs#a")
    assert doc_path("docs/A11Y_AUDIT.md#a") == "A11Y_AUDIT.md#a"
    assert slug("1.4.3 Contrast (Minimum)") == "143-contrast-minimum", slug(
        "1.4.3 Contrast (Minimum)"
    )
    assert slug("2.2.2 Pause, Stop, Hide") == "222-pause-stop-hide"

    # A missing field is a parse error, not a silent default.
    try:
        parse_rows("pub const WCAG: &[MappingRow] = &[MappingRow { criterion: \"x\" }];")
    except SourceError:
        pass
    else:
        raise AssertionError("a row with missing fields must not parse")

    # The link checker accepts a real target and rejects a broken one.
    with __import__("tempfile").TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "crates" / "example").mkdir(parents=True)
        (root / "crates" / "example" / "tests.rs").write_text(
            "fn the_test() {}\n", encoding="utf-8"
        )
        (root / "docs").mkdir()
        (root / "docs" / "A11Y_AUDIT.md").write_text(
            "### 9.9.9 Test Criterion\n", encoding="utf-8"
        )
        ok, detail = resolve_target("crates/example/tests.rs#the_test", root)
        assert ok, detail
        ok, detail = resolve_target("docs/A11Y_AUDIT.md#999-test-criterion", root)
        assert ok, detail
        ok, detail = resolve_target("crates/example/tests.rs#missing", root)
        assert not ok, detail
        ok, detail = resolve_target("crates/missing/tests.rs#the_test", root)
        assert not ok, detail
        ok, detail = resolve_target("docs/A11Y_AUDIT.md#no-such-heading", root)
        assert not ok, detail
        assert verify_links(rows, coga, root) == [], verify_links(rows, coga, root)

    print("render_a11y_mapping self-test: ok")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit 1 instead of writing when a document is stale",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="exercise the parser and the link checker",
    )
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    try:
        rendered = render_all()
    except SourceError as error:
        print(f"render_a11y_mapping: {error}", file=sys.stderr)
        return 2

    broken = verify_links(*load()[:2])
    if broken:
        for detail in broken:
            print(f"render_a11y_mapping: broken test link: {detail}", file=sys.stderr)
        return 1

    if args.check:
        stale = False
        for name, target in TARGETS.items():
            current = target.read_text(encoding="utf-8")
            if current != rendered[name]:
                print(
                    f"{target} is stale; run scripts/render_a11y_mapping.py",
                    file=sys.stderr,
                )
                stale = True
        if stale:
            return 1
        print("accessibility documents are current")
        return 0

    for name, target in TARGETS.items():
        target.write_text(rendered[name], encoding="utf-8")
        print(f"wrote {target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
