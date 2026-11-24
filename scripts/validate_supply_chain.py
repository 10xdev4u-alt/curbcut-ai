#!/usr/bin/env python3
"""Fail if the release prerequisites or the supply-chain documents regress.

CC-119 needs a published SBOM, an attestation per artifact, and reproducible
release builds. This script checks what can be checked from files alone:

* the workspace declares an MSRV, an edition, a known license, and forbids
  ``unsafe_code``;
* every crate directory is a declared member with a matching manifest and a
  resolvable license;
* ``Cargo.lock`` matches the crate set exactly -- a package with no manifest
  cannot have its license verified;
* the root license text is the AGPL it claims to be;
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

REQUIRED_FILES = (
    ("Cargo.toml", "the workspace manifest"),
    ("Cargo.lock", "the committed resolution"),
    ("LICENSE", "the license text"),
    ("docs/SUPPLY_CHAIN.md", "the release documentation"),
    (".github/workflows/ci.yml", "the release pipeline"),
)

SUPPLY_CHAIN_SECTIONS = (
    "Release recipe",
    "SBOM",
    "Provenance attestation",
    "What is not proven",
)

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


def section(text: str, heading: str) -> str:
    pattern = rf"^##\s+{re.escape(heading)}[^\n]*$"
    match = re.search(pattern, text, re.MULTILINE)
    if match is None:
        return ""
    rest = text[match.end() :]
    following = re.search(r"^##\s+", rest, re.MULTILINE)
    return rest[: following.start()] if following else rest


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

    lock_text = (root / "Cargo.lock").read_text(encoding="utf-8")
    lock = tomllib.loads(lock_text)
    locked = {entry["name"]: entry["version"] for entry in lock.get("package", [])}
    for path in declared:
        crate_manifest_path = path / "Cargo.toml"
        if not crate_manifest_path.exists():
            errors.append(f"crates/{path.name} has no Cargo.toml")
            continue
        crate_manifest = load_toml(crate_manifest_path)
        name = crate_manifest.get("package", {}).get("name")
        if name != path.name:
            errors.append(
                f"crates/{path.name} declares package name {name!r}"
            )
        license_id = resolve_license(crate_manifest, defaults)
        if license_id not in SPDX_LICENSES:
            errors.append(
                f"crates/{path.name} license {license_id!r} is not a known SPDX id"
            )
        if name not in locked:
            errors.append(f"crates/{path.name} is missing from Cargo.lock")

    for name, version in sorted(locked.items()):
        if not (root / "crates" / name / "Cargo.toml").exists():
            errors.append(
                f"{name}@{version} is in Cargo.lock but has no crate directory, "
                "so its license cannot be verified"
            )

    license_text = (root / "LICENSE").read_text(encoding="utf-8")
    lowered_license = flatten(license_text)
    # Check the title line, not the phrase: the body of the AGPL repeats the
    # name, so a search for the words would pass on a swapped document.
    lines = [line.strip() for line in license_text.splitlines() if line.strip()]
    if not lines or lines[0].upper() != "GNU AFFERO GENERAL PUBLIC LICENSE":
        errors.append("LICENSE is not the AGPL")
    if "version 3" not in lowered_license:
        errors.append("LICENSE does not state version 3")

    doc = (root / "docs" / "SUPPLY_CHAIN.md").read_text(encoding="utf-8")
    for heading in SUPPLY_CHAIN_SECTIONS:
        if not re.search(
            rf"^##\s+{re.escape(heading)}\b", doc, re.IGNORECASE | re.MULTILINE
        ):
            errors.append(f"supply-chain document is missing the {heading} section")
    body = flatten(doc)
    for phrase in (
        "--locked",
        "cyclonedx",
        "unsigned placeholder",
        "have not been shown to match",
    ):
        if phrase not in body:
            errors.append(f"supply-chain document no longer states {phrase!r}")

    workflow = (root / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
    if not re.search(r"^  sbom:\s*$", workflow, re.MULTILINE):
        errors.append("CI has no sbom job")
    return errors


def _mutated_copy(root: Path) -> Path:
    tmp = Path(tempfile.mkdtemp(prefix="curbcut-supply-selftest-"))
    relative_paths = [
        "Cargo.toml",
        "Cargo.lock",
        "LICENSE",
        "docs/SUPPLY_CHAIN.md",
        ".github/workflows/ci.yml",
    ]
    crates = root / "crates"
    if crates.is_dir():
        relative_paths.extend(
            str(path.relative_to(root)) for path in sorted(crates.glob("*/Cargo.toml"))
        )
    for relative in relative_paths:
        source = root / relative
        if not source.exists():
            continue
        target = tmp / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
    return tmp


def _break(document: Path, old: str, new: str) -> None:
    text = document.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"self-test fixture no longer matches {document.name}: {old!r}")
    document.write_text(text.replace(old, new, 1), encoding="utf-8")


Mutation = tuple[str, str, str, str, str]

MUTATIONS: tuple[Mutation, ...] = (
    ("msrv", "Cargo.toml", 'rust-version = "1.85"', 'msrv = "1.85"', "rust-version"),
    ("edition", "Cargo.toml", 'edition = "2021"\n', "", "does not declare an edition"),
    (
        "unsafe policy",
        "Cargo.toml",
        'unsafe_code = "forbid"',
        'unsafe_code = "allow"',
        "must forbid unsafe_code",
    ),
    (
        "workspace license",
        "Cargo.toml",
        'license = "AGPL-3.0-only"',
        'license = "Proprietary"',
        "is not a known SPDX id",
    ),
    (
        "workspace membership",
        "Cargo.toml",
        'members = ["crates/*"]',
        "members = []",
        "is not covered by the workspace members",
    ),
    (
        "crate package name",
        "crates/curbcut-core/Cargo.toml",
        'name = "curbcut-core"',
        'name = "curbcut-wrong"',
        "declares package name",
    ),
    (
        "lock drift",
        "Cargo.lock",
        '[[package]]\nname = "curbcut-a11y"',
        '[[package]]\nname = "serde"\nversion = "1.0.0"\n\n'
        '[[package]]\nname = "curbcut-a11y"',
        "has no crate directory",
    ),
    (
        "license text",
        "LICENSE",
        "GNU AFFERO GENERAL PUBLIC LICENSE",
        "GNU GENERAL PUBLIC LICENSE",
        "LICENSE is not the AGPL",
    ),
    (
        "release section",
        "docs/SUPPLY_CHAIN.md",
        "## Release recipe",
        "## Recipe",
        "missing the Release recipe section",
    ),
    (
        "locked build command",
        "docs/SUPPLY_CHAIN.md",
        "cargo build --release --locked",
        "cargo build --release",
        "'--locked'",
    ),
    (
        "sbom section",
        "docs/SUPPLY_CHAIN.md",
        "## SBOM",
        "## Inventory",
        "missing the SBOM section",
    ),
    (
        "sbom format",
        "docs/SUPPLY_CHAIN.md",
        "CycloneDX-shaped",
        "JSON-shaped",
        "'cyclonedx'",
    ),
    (
        "attestation section",
        "docs/SUPPLY_CHAIN.md",
        "## Provenance attestation",
        "## Attestation placeholder",
        "missing the Provenance attestation section",
    ),
    (
        "unsigned honesty",
        "docs/SUPPLY_CHAIN.md",
        "unsigned\nplaceholder",
        "pending signature",
        "'unsigned placeholder'",
    ),
    (
        "unproven section",
        "docs/SUPPLY_CHAIN.md",
        "## What is not proven",
        "## Proven claims",
        "missing the What is not proven section",
    ),
    (
        "reproducibility claim",
        "docs/SUPPLY_CHAIN.md",
        "Two independent builds have not been shown to match by hash",
        "Two independent builds match by hash",
        "'have not been shown to match'",
    ),
    (
        "ci sbom job",
        ".github/workflows/ci.yml",
        "\n  sbom:",
        "\n  bom:",
        "CI has no sbom job",
    ),
)


def self_test() -> int:
    root = _mutated_copy(ROOT)
    try:
        if validate(root):
            print("SELF-TEST FAILED: a pristine copy did not pass", file=sys.stderr)
            return 1
        print("self-test: the committed files pass")

        documents = sorted({root / relative for _, relative, _, _, _ in MUTATIONS})
        pristine = {
            document: document.read_text(encoding="utf-8") for document in documents
        }
        for label, relative, old, new, expected in MUTATIONS:
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

        (root / "Cargo.lock").unlink()
        if not validate(root):
            print("SELF-TEST FAILED: a missing lockfile was not reported", file=sys.stderr)
            return 1

        print(
            f"self-test: {len(MUTATIONS)} named deletions were each detected by "
            "their own check, and a missing lockfile failed"
        )
        return 0
    finally:
        shutil.rmtree(root, ignore_errors=True)


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
            document = json.loads(args.sbom.read_text(encoding="utf-8"))
            unknown = sorted(
                entry["name"]
                for entry in document.get("components", [])
                if entry.get("licenses", [{}])[0].get("license", {}).get("name")
                == "UNKNOWN"
            )
            if unknown:
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
