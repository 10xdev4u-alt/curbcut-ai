#!/usr/bin/env python3
"""Fail if the published fixture dataset or its documentation regresses.

CC-116 publishes the fixture corpus, its license and consent status, and the
deterministic benchmark published alongside it (which reads no corpus file).
This script checks the publication from files alone:

* ``manifest.json`` parses and every listed file exists with the recorded size
  and SHA-256;
* no file sits in the dataset directory that the manifest does not list;
* the declared license is one the repository is willing to publish;
* the canary corpus still contains canary lines, so a leak stays detectable;
* the dataset card and ``docs/DATASET.md`` keep their required sections and
  their honest statements -- synthetic content, no screen captures, consent
  not required, and a protocol citation that is explicitly *not* a user study;
* CI runs a ``dataset`` job, so the check happens on every change.

Usage::

    python3 scripts/validate_dataset.py
    python3 scripts/validate_dataset.py --self-test

This checks the publication metadata, not the benchmark timing. Run
``scripts/reproduce_benchmark.py`` for the measurement.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATASET_DIR = ROOT / "crates" / "curbcut-core" / "tests" / "fixtures"
MANIFEST = DATASET_DIR / "manifest.json"
CARD = DATASET_DIR / "README.md"
DATASET_DOC = ROOT / "docs" / "DATASET.md"
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"

REQUIRED_FILES = (
    (CARD, "the dataset card"),
    (MANIFEST, "the machine-readable inventory"),
    (DATASET_DOC, "the researcher-facing page"),
    (WORKFLOW, "the release pipeline"),
)

CARD_SECTIONS = (
    "License",
    "Consent",
    "Contents",
    "Reproducing the benchmark",
    "Protocol citation",
    "Limits",
)

CARD_PHRASES = (
    "cc0-1.0-only",
    "synthetic",
    "no screen capture",
    "not-required",
)

DOC_PHRASES = (
    "cc0-1.0-only",
    "synthetic",
    "small and synthetic",
    "no screen capture",
    "study_protocol.md",
    "reproduce_benchmark.py",
    "must not be cited as evidence about users",
    "--from build/bench.txt",
)

PUBLISHABLE_LICENSES = {"CC0-1.0-only"}
CANARY_MARKER = "CC-CANARY-"
# Sentinel for a self-test case that removes a key from a JSON document.
DELETE = object()


def flatten(text: str) -> str:
    return " ".join(text.lower().split())


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65_536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def section(text: str, heading: str) -> str:
    pattern = rf"^##\s+{re.escape(heading)}[^\n]*$"
    match = re.search(pattern, text, re.MULTILINE)
    if match is None:
        return ""
    rest = text[match.end() :]
    following = re.search(r"^##\s+", rest, re.MULTILINE)
    return rest[: following.start()] if following else rest


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    card = root / CARD.relative_to(ROOT)
    manifest_path = root / MANIFEST.relative_to(ROOT)
    doc = root / DATASET_DOC.relative_to(ROOT)
    workflow = root / WORKFLOW.relative_to(ROOT)
    dataset_dir = root / DATASET_DIR.relative_to(ROOT)

    for path, why in REQUIRED_FILES:
        if not (root / path.relative_to(ROOT)).exists():
            errors.append(f"missing {path.relative_to(ROOT)} ({why})")
    if errors:
        return errors

    try:
        raw_manifest = manifest_path.read_text(encoding="utf-8")
    except OSError as error:
        return [f"{MANIFEST.relative_to(ROOT)}: cannot read the manifest: {error}"]
    try:
        manifest = json.loads(raw_manifest)
    except json.JSONDecodeError as error:
        return [f"{MANIFEST.relative_to(ROOT)}: invalid JSON: {error}"]
    if not isinstance(manifest, dict):
        return [f"{MANIFEST.relative_to(ROOT)}: expected a JSON object"]

    if manifest.get("name") != "curbcut-fixtures":
        errors.append("manifest name is not curbcut-fixtures")
    if not isinstance(manifest.get("version"), str) or not manifest["version"]:
        errors.append("manifest has no version")
    license_id = manifest.get("license")
    if license_id not in PUBLISHABLE_LICENSES:
        errors.append(
            f"manifest license {license_id!r} is not one the repository publishes"
        )
    if manifest.get("consent") != "not-required":
        errors.append("manifest consent is not 'not-required'")
    note = flatten(str(manifest.get("consent_note", "")))
    if "synthetic" not in note:
        errors.append("manifest consent_note does not state the corpus is synthetic")

    entries = manifest.get("files")
    if not isinstance(entries, list) or not entries:
        return errors + ["manifest lists no files"]

    listed: set[str] = set()
    for entry in entries:
        if not isinstance(entry, dict):
            errors.append("manifest has a file entry that is not an object")
            continue
        relative = entry.get("path")
        if not isinstance(relative, str) or not relative:
            errors.append("manifest has a file entry with no path")
            continue
        if Path(relative).is_absolute() or ".." in Path(relative).parts:
            errors.append(f"manifest path {relative!r} escapes the dataset")
            continue
        listed.add(relative)
        path = dataset_dir / relative
        if not path.exists():
            errors.append(f"{relative} is listed in the manifest but missing")
            continue
        size = path.stat().st_size
        if entry.get("bytes") != size:
            errors.append(
                f"{relative} is {size} bytes, manifest says {entry.get('bytes')!r}"
            )
        digest = sha256_of(path)
        if entry.get("sha256") != digest:
            errors.append(f"{relative} sha256 differs from the manifest")
        role = entry.get("role")
        if not isinstance(role, str) or not role.strip():
            errors.append(f"{relative} has no recorded role")

    present = {
        str(path.relative_to(dataset_dir))
        for path in sorted(dataset_dir.rglob("*"))
        if path.is_file()
    }
    for extra in sorted(present - listed - {"README.md", "manifest.json"}):
        errors.append(f"{extra} is in the dataset but not listed in the manifest")

    canary = dataset_dir / "content_canaries.txt"
    if canary.exists():
        lines = canary.read_text(encoding="utf-8").splitlines()
        if not any(line.startswith(CANARY_MARKER) for line in lines):
            errors.append("the canary corpus contains no canary lines")
        offenders = [
            line
            for line in lines
            if line.strip()
            and not line.startswith("#")
            and not line.startswith(CANARY_MARKER)
        ]
        if offenders:
            errors.append(
                "the canary corpus has a content line that is not a canary: "
                + offenders[0][:40]
            )

    benchmark = manifest.get("benchmark")
    if not isinstance(benchmark, dict):
        errors.append("manifest has no benchmark block")
    else:
        # ``type`` rather than ``isinstance``: a JSON boolean is an int subclass
        # in Python and would otherwise pass as iterations=1.
        if type(benchmark.get("iterations")) is not int or benchmark["iterations"] <= 0:
            errors.append("manifest benchmark iterations is not a positive integer")
        if (
            type(benchmark.get("expected_bytes")) is not int
            or benchmark["expected_bytes"] <= 0
        ):
            errors.append("manifest benchmark expected_bytes is not a positive integer")
        if benchmark.get("command") != "cargo run -q -p curbcut-cli -- bench":
            errors.append("manifest benchmark command changed")

    card_text = card.read_text(encoding="utf-8")
    for heading in CARD_SECTIONS:
        if not section(card_text, heading):
            errors.append(f"dataset card is missing the {heading} section")
    card_body = flatten(card_text)
    for phrase in CARD_PHRASES:
        if phrase not in card_body:
            errors.append(f"dataset card no longer states {phrase!r}")

    doc_body = flatten(doc.read_text(encoding="utf-8"))
    for phrase in DOC_PHRASES:
        if phrase not in doc_body:
            errors.append(f"docs/DATASET.md no longer states {phrase!r}")

    if not re.search(r"^  dataset:\s*$", workflow.read_text(encoding="utf-8"), re.MULTILINE):
        errors.append("CI has no dataset job")
    return errors


def collected_files(root: Path) -> list[str]:
    paths = [CARD, MANIFEST, DATASET_DOC, WORKFLOW]
    if DATASET_DIR.is_dir():
        paths.extend(path for path in sorted(DATASET_DIR.rglob("*")) if path.is_file())
    return [str(path.relative_to(root)) for path in paths]


def materialize(target: Path, pristine: dict[str, str]) -> None:
    if target.exists():
        shutil.rmtree(target)
    for relative, text in pristine.items():
        path = target / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


CASES: tuple[dict[str, object], ...] = (
    {
        "label": "dataset card deleted",
        "delete": [str(CARD.relative_to(ROOT))],
        "expect": "missing crates/curbcut-core/tests/fixtures/README.md",
    },
    {
        "label": "manifest deleted",
        "delete": [str(MANIFEST.relative_to(ROOT))],
        "expect": "missing crates/curbcut-core/tests/fixtures/manifest.json",
    },
    {
        "label": "researcher page deleted",
        "delete": ["docs/DATASET.md"],
        "expect": "missing docs/DATASET.md",
    },
    {
        "label": "workflow deleted",
        "delete": [".github/workflows/ci.yml"],
        "expect": "missing .github/workflows/ci.yml",
    },
    {
        "label": "manifest is not valid JSON",
        "edit": [(str(MANIFEST.relative_to(ROOT)), "{", "{not json")],
        "expect": "invalid JSON",
    },
    {
        "label": "listed file deleted",
        "delete": ["crates/curbcut-core/tests/fixtures/content_canaries.txt"],
        "expect": "is listed in the manifest but missing",
    },
    {
        "label": "unlisted file added",
        "create": [
            (
                "crates/curbcut-core/tests/fixtures/stray.txt",
                "not part of the publication\n",
            )
        ],
        "expect": "not listed in the manifest",
    },
    {
        "label": "manifest hash drifted",
        "edit": [
            (
                str(MANIFEST.relative_to(ROOT)),
                "834008eab783c255370f634c0810e9f87e2784db59cb492cd17f4e2ff6e421ea",
                "934008eab783c255370f634c0810e9f87e2784db59cb492cd17f4e2ff6e421ea",
            )
        ],
        "expect": "sha256 differs",
    },
    {
        "label": "manifest size drifted",
        "edit": [
            (str(MANIFEST.relative_to(ROOT)), '"bytes": 757', '"bytes": 758')
        ],
        "expect": "bytes, manifest says",
    },
    {
        "label": "license changed",
        "edit": [
            (str(MANIFEST.relative_to(ROOT)), '"CC0-1.0-only"', '"Proprietary"')
        ],
        "expect": "not one the repository publishes",
    },
    {
        "label": "consent changed",
        "edit": [(str(MANIFEST.relative_to(ROOT)), '"not-required"', '"required"')],
        "expect": "consent is not",
    },
    {
        "label": "consent note loses synthetic statement",
        "edit": [
            (
                str(MANIFEST.relative_to(ROOT)),
                "Every file is synthetic and authored for this repository.",
                "Files were collected from contributors.",
            )
        ],
        "expect": "synthetic",
    },
    {
        "label": "benchmark block lost",
        "json": [(str(MANIFEST.relative_to(ROOT)), "benchmark", DELETE)],
        "expect": "manifest has no benchmark block",
    },
    {
        "label": "expected byte count invalid",
        "json": [(str(MANIFEST.relative_to(ROOT)), "benchmark.expected_bytes", -1)],
        "expect": "expected_bytes is not a positive integer",
    },
    {
        "label": "canary line lost",
        "edit": [
            (
                "crates/curbcut-core/tests/fixtures/content_canaries.txt",
                "CC-CANARY-",
                "CC-REDHERRING-",
            )
        ],
        "expect": "not a canary",
    },
    {
        "label": "card license section",
        "edit": [(str(CARD.relative_to(ROOT)), "## License", "## Terms")],
        "expect": "missing the License section",
    },
    {
        "label": "card consent section",
        "edit": [(str(CARD.relative_to(ROOT)), "## Consent", "## Permission")],
        "expect": "missing the Consent section",
    },
    {
        "label": "card contents section",
        "edit": [(str(CARD.relative_to(ROOT)), "## Contents", "## Files")],
        "expect": "missing the Contents section",
    },
    {
        "label": "card reproduction section",
        "edit": [
            (str(CARD.relative_to(ROOT)), "## Reproducing the benchmark", "## Running it")
        ],
        "expect": "missing the Reproducing the benchmark section",
    },
    {
        "label": "card protocol section",
        "edit": [(str(CARD.relative_to(ROOT)), "## Protocol citation", "## References")],
        "expect": "missing the Protocol citation section",
    },
    {
        "label": "card limits section",
        "edit": [(str(CARD.relative_to(ROOT)), "## Limits", "## Caveats")],
        "expect": "missing the Limits section",
    },
    {
        "label": "card license phrase",
        "edit": [(str(CARD.relative_to(ROOT)), "CC0-1.0-only", "AGPL-3.0-only")],
        "expect": "'cc0-1.0-only'",
    },
    {
        "label": "card consent phrase",
        "edit": [
            (str(CARD.relative_to(ROOT)), "no screen capture", "some screen captures")
        ],
        "expect": "'no screen capture'",
    },
    {
        "label": "card not-required phrase",
        "edit": [(str(CARD.relative_to(ROOT)), "`not-required`", "`required`")],
        "expect": "'not-required'",
    },
    {
        "label": "doc synthetic phrase",
        "edit": [
            ("docs/DATASET.md", "small and synthetic", "small and real")
        ],
        "expect": "'small and synthetic'",
    },
    {
        "label": "doc no-screen-capture phrase",
        "edit": [("docs/DATASET.md", "no screen capture", "some screen captures")],
        "expect": "'no screen capture'",
    },
    {
        "label": "doc protocol link",
        "edit": [
            (
                "docs/DATASET.md",
                "not be cited as evidence about",
                "not be cited as proof about",
            )
        ],
        "expect": "'must not be cited as evidence about users'",
    },
    {
        "label": "doc harness link",
        "edit": [
            ("docs/DATASET.md", "--from build/bench.txt", "--from build/other.txt")
        ],
        "expect": "'--from build/bench.txt'",
    },
    {
        "label": "ci dataset job",
        "edit": [(".github/workflows/ci.yml", "\n  dataset:", "\n  data:")],
        "expect": "CI has no dataset job",
    },
    {
        "label": "manifest is an array",
        "write": [(str(MANIFEST.relative_to(ROOT)), "[]")],
        "expect": "expected a JSON object",
    },
    {
        "label": "manifest name changed",
        "json": [(str(MANIFEST.relative_to(ROOT)), "name", "curbcut-wrong")],
        "expect": "manifest name is not curbcut-fixtures",
    },
    {
        "label": "manifest version empty",
        "json": [(str(MANIFEST.relative_to(ROOT)), "version", "")],
        "expect": "manifest has no version",
    },
    {
        "label": "manifest lists no files",
        "json": [(str(MANIFEST.relative_to(ROOT)), "files", [])],
        "expect": "manifest lists no files",
    },
    {
        "label": "file entry is not an object",
        "json": [
            (str(MANIFEST.relative_to(ROOT)), "files", ["content_canaries.txt"])
        ],
        "expect": "file entry that is not an object",
    },
    {
        "label": "file entry has no path",
        "json": [(str(MANIFEST.relative_to(ROOT)), "files", [{}])],
        "expect": "file entry with no path",
    },
    {
        "label": "file path escapes the dataset",
        "json": [
            (
                str(MANIFEST.relative_to(ROOT)),
                "files.0.path",
                "../evil.txt",
            )
        ],
        "expect": "escapes the dataset",
    },
    {
        "label": "file entry has no role",
        "json": [(str(MANIFEST.relative_to(ROOT)), "files.0.role", DELETE)],
        "expect": "has no recorded role",
    },
    {
        "label": "benchmark iterations invalid",
        "json": [(str(MANIFEST.relative_to(ROOT)), "benchmark.iterations", -1)],
        "expect": "iterations is not a positive integer",
    },
    {
        "label": "benchmark iterations is a boolean",
        "json": [(str(MANIFEST.relative_to(ROOT)), "benchmark.iterations", True)],
        "expect": "iterations is not a positive integer",
    },
    {
        "label": "expected byte count is a boolean",
        "json": [
            (str(MANIFEST.relative_to(ROOT)), "benchmark.expected_bytes", True)
        ],
        "expect": "expected_bytes is not a positive integer",
    },
    {
        "label": "file role is not a string",
        "json": [(str(MANIFEST.relative_to(ROOT)), "files.0.role", 123)],
        "expect": "has no recorded role",
    },
    {
        "label": "benchmark command changed",
        "json": [
            (
                str(MANIFEST.relative_to(ROOT)),
                "benchmark.command",
                "cargo run -q -p curbcut-cli -- other",
            )
        ],
        "expect": "benchmark command changed",
    },
    {
        "label": "canary corpus emptied",
        "write": [("crates/curbcut-core/tests/fixtures/content_canaries.txt", "")],
        "expect": "contains no canary lines",
    },
)


def _apply_json(path: Path, dotted: str, value: object) -> None:
    """Set or delete a key in a JSON document addressed by a dotted path.

    ``DELETE`` removes the key. The rewrite uses ``json.dumps`` so a case that
    mutates the manifest never depends on its original formatting.
    """
    document = json.loads(path.read_text(encoding="utf-8"))
    keys = dotted.split(".")
    node: object = document
    for key in keys[:-1]:
        node = node[int(key)] if isinstance(node, list) else node[key]
    last = keys[-1]
    if value is DELETE:
        if isinstance(node, list):
            del node[int(last)]
        else:
            del node[last]
    elif isinstance(node, list):
        node[int(last)] = value
    else:
        node[last] = value
    path.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")


def self_test() -> int:
    pristine = {
        relative: (ROOT / relative).read_text(encoding="utf-8")
        for relative in collected_files(ROOT)
    }
    tmp = Path(tempfile.mkdtemp(prefix="curbcut-dataset-selftest-"))
    try:
        materialize(tmp, pristine)
        if validate(tmp):
            print("SELF-TEST FAILED: a pristine copy did not pass", file=sys.stderr)
            return 1
        print("self-test: the committed files pass")

        for case in CASES:
            materialize(tmp, pristine)
            for relative in case.get("delete", ()):  # type: ignore[union-attr]
                (tmp / str(relative)).unlink()
            for relative, old, new in case.get("edit", ()):  # type: ignore[union-attr]
                path = tmp / str(relative)
                text = path.read_text(encoding="utf-8")
                if old not in text:
                    print(
                        f"SELF-TEST FIXTURE BROKEN: {case['label']} cannot find "
                        f"{old!r} in {relative}",
                        file=sys.stderr,
                    )
                    return 1
                path.write_text(text.replace(old, new, 1), encoding="utf-8")
            for relative, content in case.get("create", ()):  # type: ignore[union-attr]
                path = tmp / str(relative)
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(str(content), encoding="utf-8")
            for relative, dotted, value in case.get("json", ()):  # type: ignore[union-attr]
                _apply_json(tmp / str(relative), str(dotted), value)
            for relative, content in case.get("write", ()):  # type: ignore[union-attr]
                path = tmp / str(relative)
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(str(content), encoding="utf-8")

            errors = validate(tmp)
            expected = case["expect"]
            if not any(str(expected) in error for error in errors):
                print(
                    f"SELF-TEST FAILED: {case['label']} was not detected by its "
                    f"own check; wanted {expected!r}, got {errors[:2]}",
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
    parser.add_argument("--self-test", action="store_true", help="run built-in checks")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    errors = validate(ROOT)
    for message in errors:
        print(f"FAIL: {message}", file=sys.stderr)
    if errors:
        print(f"{len(errors)} dataset check(s) failed", file=sys.stderr)
        return 1
    print("the published fixture dataset and its documentation are intact")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
