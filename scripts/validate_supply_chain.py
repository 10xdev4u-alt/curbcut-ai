#!/usr/bin/env python3
"""Fail if the release prerequisites or the supply-chain documents regress.

CC-119 needs a published SBOM, an attestation per artifact, and reproducible
release builds. This script checks what can be checked from files alone:

* the workspace declares an MSRV, an edition, a known license, and forbids
  ``unsafe_code``;
* every crate directory is a declared member with a matching manifest and a
  resolvable license;
* ``Cargo.lock`` matches the crate set exactly, keyed by name and version;
* a locked package outside the workspace has a recorded license in
  ``supply-chain/registry-licenses.json`` with a known SPDX id;
* the root license carries the AGPL title, its version 3 title line, and a
  distinctive sentence from the grant;
* ``docs/SUPPLY_CHAIN.md`` keeps its four sections and its honest statements,
  including the one that says two independent builds have *not* been shown to
  match;
* CI has an ``sbom`` job, so the artifact is actually produced.

Usage::

    python3 scripts/validate_supply_chain.py
    python3 scripts/validate_supply_chain.py --sbom build/sbom.json
    python3 scripts/validate_supply_chain.py --self-test

This checks declared inputs, not built binaries. It cannot prove a hash matches
or that a signature is valid; ``docs/SUPPLY_CHAIN.md`` says so.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import re
import shutil
import sys
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REGISTRY_LICENSES = "supply-chain/registry-licenses.json"

REQUIRED_FILES = (
    ("Cargo.toml", "the workspace manifest"),
    ("Cargo.lock", "the committed resolution"),
    ("LICENSE", "the license text"),
    ("docs/SUPPLY_CHAIN.md", "the release documentation"),
    (REGISTRY_LICENSES, "the third-party license declarations"),
    (".github/workflows/ci.yml", "the release pipeline"),
)

SUPPLY_CHAIN_SECTIONS = (
    "Release recipe",
    "SBOM",
    "Provenance attestation",
    "What is not proven",
)

SUPPLY_CHAIN_PHRASES = (
    "--locked",
    "cyclonedx",
    "unsigned placeholder",
    "have not been shown to match",
    "registry-licenses.json",
    "never invents a license",
)

LICENSE_BODY_ANCHOR = "Everyone is permitted to copy and distribute verbatim copies"

SPDX_LICENSES = {
    "0BSD",
    "AGPL-3.0-only",
    "AGPL-3.0-or-later",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "BSL-1.0",
    "CC0-1.0",
    "GPL-2.0-only",
    "GPL-3.0-only",
    "ISC",
    "LGPL-2.1-only",
    "LGPL-3.0-only",
    "MIT",
    "MPL-2.0",
    "Unlicense",
}


def flatten(text: str) -> str:
    return " ".join(text.lower().split())


def load_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def member_dirs(root: Path, patterns: list[str]) -> tuple[list[Path], list[Path]]:
    """Return (declared, undeclared) crate directories."""
    crates = root / "crates"
    found = (
        sorted(path for path in crates.iterdir() if path.is_dir())
        if crates.is_dir()
        else []
    )
    declared = [
        path
        for path in found
        if any(
            fnmatch.fnmatch(path.relative_to(root).as_posix(), pattern)
            for pattern in patterns
        )
    ]
    return declared, [path for path in found if path not in declared]


def resolve_license(crate_manifest: dict, defaults: dict) -> str | None:
    package = crate_manifest.get("package", {})
    value = package.get("license")
    if isinstance(value, dict):
        if value.get("workspace"):
            return defaults.get("license")
        value = value.get("file")
        return f"file:{value}" if value else None
    if value is None:
        value = defaults.get("license")
    return value


def load_registry_licenses(root: Path) -> dict[str, str]:
    """Read recorded licenses for packages that live outside the workspace."""
    try:
        document = json.loads((root / REGISTRY_LICENSES).read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ValueError(f"{REGISTRY_LICENSES}: invalid JSON: {error}") from error
    if not isinstance(document, dict):
        raise ValueError(f"{REGISTRY_LICENSES}: expected a JSON object")
    return {str(key): str(value) for key, value in document.items()}


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    for relative, why in REQUIRED_FILES:
        if not (root / relative).exists():
            errors.append(f"missing {relative} ({why})")

    if errors:
        return errors

    manifest = load_toml(root / "Cargo.toml")
    workspace = manifest.get("workspace", {})
    defaults = workspace.get("package", {})
    if not defaults.get("rust-version"):
        errors.append("workspace does not declare rust-version (the MSRV)")
    if not defaults.get("edition"):
        errors.append("workspace does not declare an edition")
    lints = workspace.get("lints", {}).get("rust", {})
    if lints.get("unsafe_code") != "forbid":
        errors.append("workspace must forbid unsafe_code")
    if defaults.get("license") not in SPDX_LICENSES:
        errors.append(
            f"workspace license {defaults.get('license')!r} is not a known SPDX id"
        )

    patterns = workspace.get("members", [])
    declared, undeclared = member_dirs(root, patterns)
    for path in undeclared:
        errors.append(f"crates/{path.name} is not covered by the workspace members")

    lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
    locked_pairs = [
        (entry["name"], entry["version"]) for entry in lock.get("package", [])
    ]
    if len(locked_pairs) != len(set(locked_pairs)):
        errors.append("Cargo.lock lists the same package name and version twice")
    locked_names = {name for name, _ in locked_pairs}

    for path in declared:
        crate_manifest_path = path / "Cargo.toml"
        if not crate_manifest_path.exists():
            errors.append(f"crates/{path.name} has no Cargo.toml")
            continue
        crate_manifest = load_toml(crate_manifest_path)
        name = crate_manifest.get("package", {}).get("name")
        if name != path.name:
            errors.append(f"crates/{path.name} declares package name {name!r}")
        license_id = resolve_license(crate_manifest, defaults)
        if license_id not in SPDX_LICENSES:
            errors.append(
                f"crates/{path.name} license {license_id!r} is not a known SPDX id"
            )
        if name not in locked_names:
            errors.append(f"crates/{path.name} is missing from Cargo.lock")

    try:
        registry = load_registry_licenses(root)
    except ValueError as error:
        errors.append(str(error))
        registry = {}

    for name, version in sorted(set(locked_pairs)):
        if (root / "crates" / name).is_dir():
            continue
        recorded = registry.get(f"{name}@{version}") or registry.get(name)
        if recorded is None:
            errors.append(
                f"{name}@{version} has no recorded license in {REGISTRY_LICENSES}"
            )
        elif recorded not in SPDX_LICENSES:
            errors.append(
                f"{name}@{version} recorded license {recorded!r} is not a known "
                "SPDX id"
            )

    license_text = (root / "LICENSE").read_text(encoding="utf-8")
    lines = [line.strip() for line in license_text.splitlines() if line.strip()]
    # Check the title and its version line, not the words: the body of the AGPL
    # repeats both, so a search for the phrase would pass on a swapped document.
    if not lines or lines[0].upper() != "GNU AFFERO GENERAL PUBLIC LICENSE":
        errors.append("LICENSE is not the AGPL")
    if len(lines) < 2 or lines[1] != "Version 3, 19 November 2007":
        errors.append("LICENSE does not state the AGPL version 3 title line")
    # The header alone is easy to keep while the body is replaced, so a
    # distinctive sentence from the grant is required too.
    if LICENSE_BODY_ANCHOR not in license_text:
        errors.append("LICENSE lost its verbatim-distribution permission line")

    doc = (root / "docs" / "SUPPLY_CHAIN.md").read_text(encoding="utf-8")
    for heading in SUPPLY_CHAIN_SECTIONS:
        if not re.search(
            rf"^##\s+{re.escape(heading)}\b", doc, re.IGNORECASE | re.MULTILINE
        ):
            errors.append(f"supply-chain document is missing the {heading} section")
    body = flatten(doc)
    for phrase in SUPPLY_CHAIN_PHRASES:
        if phrase not in body:
            errors.append(f"supply-chain document no longer states {phrase!r}")

    workflow = (root / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
    if not re.search(r"^  sbom:\s*$", workflow, re.MULTILINE):
        errors.append("CI has no sbom job")
    return errors


def collected_files(root: Path) -> list[str]:
    relative = [path for path, _ in REQUIRED_FILES]
    crates = root / "crates"
    if crates.is_dir():
        relative.extend(
            str(path.relative_to(root)) for path in sorted(crates.glob("*/Cargo.toml"))
        )
    return relative


def materialize(target: Path, pristine: dict[str, str]) -> None:
    """Recreate the pristine tree from bytes held in memory."""
    if target.exists():
        shutil.rmtree(target)
    for relative, text in pristine.items():
        path = target / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


# Each case names the files it deletes, the replacements it makes, the files it
# creates, and the error its own check must produce. ``expect`` of None means
# the case must produce no error at all.
Case = dict[str, object]

INSERT_SERDE = (
    '[[package]]\nname = "serde"\nversion = "1.0.200"\n\n'
    '[[package]]\nname = "curbcut-a11y"'
)
AT_A11Y = '[[package]]\nname = "curbcut-a11y"'

CASES: tuple[Case, ...] = (
    {
        "label": "workspace manifest deleted",
        "delete": ["Cargo.toml"],
        "expect": "missing Cargo.toml",
    },
    {
        "label": "lockfile deleted",
        "delete": ["Cargo.lock"],
        "expect": "missing Cargo.lock",
    },
    {
        "label": "license text deleted",
        "delete": ["LICENSE"],
        "expect": "missing LICENSE",
    },
    {
        "label": "supply-chain document deleted",
        "delete": ["docs/SUPPLY_CHAIN.md"],
        "expect": "missing docs/SUPPLY_CHAIN.md",
    },
    {
        "label": "registry declaration deleted",
        "delete": [REGISTRY_LICENSES],
        "expect": f"missing {REGISTRY_LICENSES}",
    },
    {
        "label": "CI pipeline deleted",
        "delete": [".github/workflows/ci.yml"],
        "expect": "missing .github/workflows/ci.yml",
    },
    {
        "label": "crate manifest deleted",
        "delete": ["crates/curbcut-core/Cargo.toml"],
        "expect": "crates/curbcut-core has no Cargo.toml",
    },
    {
        "label": "msrv",
        "edit": [("Cargo.toml", 'rust-version = "1.85"', 'msrv = "1.85"')],
        "expect": "rust-version",
    },
    {
        "label": "edition",
        "edit": [("Cargo.toml", 'edition = "2021"\n', "")],
        "expect": "does not declare an edition",
    },
    {
        "label": "unsafe policy",
        "edit": [("Cargo.toml", 'unsafe_code = "forbid"', 'unsafe_code = "allow"')],
        "expect": "must forbid unsafe_code",
    },
    {
        "label": "workspace license",
        "edit": [
            ("Cargo.toml", 'license = "AGPL-3.0-only"', 'license = "Proprietary"')
        ],
        "expect": "workspace license 'Proprietary' is not a known SPDX id",
    },
    {
        "label": "workspace membership",
        "edit": [("Cargo.toml", 'members = ["crates/*"]', "members = []")],
        "expect": "is not covered by the workspace members",
    },
    {
        "label": "crate package name",
        "edit": [
            ("crates/curbcut-core/Cargo.toml", 'name = "curbcut-core"', 'name = "curbcut-wrong"')
        ],
        "expect": "declares package name",
    },
    {
        "label": "crate license unknown",
        "edit": [
            (
                "crates/curbcut-core/Cargo.toml",
                "license.workspace = true",
                'license = "Proprietary"',
            )
        ],
        "expect": "is not a known SPDX id",
    },
    {
        "label": "crate missing from lock",
        "edit": [
            (
                "Cargo.lock",
                '[[package]]\nname = "curbcut-core"\nversion = "0.1.0"',
                '[[package]]\nname = "curbcut-moved"\nversion = "0.1.0"',
            )
        ],
        "expect": "is missing from Cargo.lock",
    },
    {
        "label": "duplicate lock entry",
        "edit": [
            (
                "Cargo.lock",
                'version = 4\n',
                'version = 4\n\n[[package]]\nname = "curbcut-a11y"\nversion = "0.1.0"\n',
            )
        ],
        "expect": "twice",
    },
    {
        "label": "unregistered external package",
        "edit": [("Cargo.lock", AT_A11Y, INSERT_SERDE)],
        "expect": "has no recorded license",
    },
    {
        "label": "external package with an unknown SPDX id",
        "edit": [
            ("Cargo.lock", AT_A11Y, INSERT_SERDE),
            (REGISTRY_LICENSES, "{}", '{"serde": "Not-A-License"}'),
        ],
        "expect": "not a known SPDX id",
    },
    {
        "label": "external package with a declared license",
        "edit": [
            ("Cargo.lock", AT_A11Y, INSERT_SERDE),
            (REGISTRY_LICENSES, "{}", '{"serde": "MIT"}'),
        ],
        "expect": None,
    },
    {
        "label": "registry file is not an object",
        "edit": [(REGISTRY_LICENSES, "{}", "[]")],
        "expect": "expected a JSON object",
    },
    {
        "label": "registry file is not valid JSON",
        "edit": [(REGISTRY_LICENSES, "{}", "{not json")],
        "expect": "invalid JSON",
    },
    {
        "label": "license text",
        "edit": [
            ("LICENSE", "GNU AFFERO GENERAL PUBLIC LICENSE", "GNU GENERAL PUBLIC LICENSE")
        ],
        "expect": "LICENSE is not the AGPL",
    },
    {
        "label": "license version line",
        "edit": [
            ("LICENSE", "Version 3, 19 November 2007", "Version 2, 19 November 2007")
        ],
        "expect": "does not state the AGPL version 3 title line",
    },
    {
        "label": "license body",
        "edit": [
            (
                "LICENSE",
                "Everyone is permitted to copy and distribute verbatim copies",
                "You may not copy this file",
            )
        ],
        "expect": "verbatim-distribution permission line",
    },
    {
        "label": "release section",
        "edit": [("docs/SUPPLY_CHAIN.md", "## Release recipe", "## Recipe")],
        "expect": "missing the Release recipe section",
    },
    {
        "label": "locked build command",
        "edit": [
            (
                "docs/SUPPLY_CHAIN.md",
                "cargo build --release --locked",
                "cargo build --release",
            )
        ],
        "expect": "'--locked'",
    },
    {
        "label": "sbom section",
        "edit": [("docs/SUPPLY_CHAIN.md", "## SBOM\n", "## Inventory\n")],
        "expect": "missing the SBOM section",
    },
    {
        "label": "sbom format",
        "edit": [("docs/SUPPLY_CHAIN.md", "CycloneDX-shaped", "JSON-shaped")],
        "expect": "'cyclonedx'",
    },
    {
        "label": "attestation section",
        "edit": [
            ("docs/SUPPLY_CHAIN.md", "## Provenance attestation", "## Attestation placeholder")
        ],
        "expect": "missing the Provenance attestation section",
    },
    {
        "label": "unsigned honesty",
        "edit": [
            ("docs/SUPPLY_CHAIN.md", "unsigned\nplaceholder", "pending signature")
        ],
        "expect": "'unsigned placeholder'",
    },
    {
        "label": "registry declaration mention",
        "edit": [
            (
                "docs/SUPPLY_CHAIN.md",
                "`supply-chain/registry-licenses.json`",
                "`supply-chain/licenses.json`",
            )
        ],
        "expect": "'registry-licenses.json'",
    },
    {
        "label": "license invention claim",
        "edit": [
            (
                "docs/SUPPLY_CHAIN.md",
                "the generator never invents a license",
                "the generator records a best guess",
            )
        ],
        "expect": "'never invents a license'",
    },
    {
        "label": "unproven section",
        "edit": [
            ("docs/SUPPLY_CHAIN.md", "## What is not proven", "## Proven claims")
        ],
        "expect": "missing the What is not proven section",
    },
    {
        "label": "reproducibility claim",
        "edit": [
            (
                "docs/SUPPLY_CHAIN.md",
                "Two independent builds have not been shown to match by hash",
                "Two independent builds match by hash",
            )
        ],
        "expect": "'have not been shown to match'",
    },
    {
        "label": "ci sbom job",
        "edit": [(".github/workflows/ci.yml", "\n  sbom:", "\n  bom:")],
        "expect": "CI has no sbom job",
    },
)


def self_test() -> int:
    pristine = {
        relative: (ROOT / relative).read_text(encoding="utf-8")
        for relative in collected_files(ROOT)
    }
    tmp = Path(tempfile.mkdtemp(prefix="curbcut-supply-selftest-"))
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

            errors = validate(tmp)
            expected = case["expect"]
            if expected is None:
                if errors:
                    print(
                        f"SELF-TEST FAILED: {case['label']} should pass but "
                        f"produced {errors[:2]}",
                        file=sys.stderr,
                    )
                    return 1
            elif not any(str(expected) in error for error in errors):
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
    parser.add_argument("--sbom", type=Path, help="also verify a generated SBOM")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    errors = validate(ROOT)
    if args.sbom:
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        import sbom  # noqa: PLC0415 - local module, stdlib only

        if not args.sbom.exists():
            errors.append(f"{args.sbom}: generated SBOM is missing")
        else:
            errors.extend(sbom.check_sbom(ROOT, args.sbom))
            try:
                document = json.loads(args.sbom.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError) as error:
                errors.append(f"{args.sbom}: invalid JSON: {error}")
            else:
                unknown = (
                    sbom.unknown_components(document)
                    if isinstance(document, dict)
                    else []
                )
                if not isinstance(document, dict):
                    errors.append(f"{args.sbom}: expected a JSON object")
                elif unknown:
                    errors.append(
                        f"{args.sbom}: components without a resolvable license: "
                        + ", ".join(unknown)
                    )

    for message in errors:
        print(f"FAIL: {message}", file=sys.stderr)
    if errors:
        print(f"{len(errors)} supply-chain check(s) failed", file=sys.stderr)
        return 1
    print("release prerequisites and supply-chain documents are intact")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
