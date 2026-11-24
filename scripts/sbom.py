#!/usr/bin/env python3
"""Generate an SBOM and a release attestation skeleton from the manifests.

CC-119 asks for a published SBOM listing every dependency and its license, plus
a provenance attestation per artifact. This script derives both from files that
are already in the repository, so it needs no network and no third-party
packages:

* ``Cargo.lock`` names every package in the build graph, including transitive
  ones, with the exact versions resolved.
* each crate's ``Cargo.toml`` carries its declared license. Workspace-inherited
  values are read from the workspace root.

The SBOM is a CycloneDX-shaped document. It deliberately carries **no**
timestamp and **no** random serial number, so two runs over the same tree
produce the same bytes; that property is part of the point, and ``--self-test``
asserts it.

A dependency whose license cannot be resolved from a manifest is recorded with
an ``UNKNOWN`` license rather than guessed, and ``--strict`` turns that into a
failure. No license is ever invented.

Usage::

    python3 scripts/sbom.py generate --output build/sbom.json \
        --attestation build/attestation.json

    python3 scripts/sbom.py check --sbom build/sbom.json

    python3 scripts/sbom.py --self-test
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORKSPACE_MANIFEST = ROOT / "Cargo.toml"
LOCKFILE = ROOT / "Cargo.lock"

SPEC_VERSION = "1.6"
TOOL_NAME = "curbcut-sbom"

# Licenses the release process is willing to publish. Anything else is UNKNOWN,
# never silently mapped.
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


class SbomError(Exception):
    """Raised when the manifests and the generated SBOM disagree."""


def load_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def workspace_defaults(manifest: dict) -> dict:
    return manifest.get("workspace", {}).get("package", {})


def workspace_members(manifest: dict) -> list[str]:
    return list(manifest.get("workspace", {}).get("members", []))


def resolve_license(crate_dir: Path, defaults: dict) -> str | None:
    """Return the declared license for one crate, or ``None`` if absent."""
    package = load_toml(crate_dir / "Cargo.toml").get("package", {})
    value = package.get("license")
    if isinstance(value, dict):
        if value.get("workspace"):
            return defaults.get("license")
        value = value.get("file")  # `license = { file = ... }`
        if value:
            return f"file:{value}"
    if value is None:
        value = defaults.get("license")
    return value


def cargo_lock_packages(lock_text: str) -> dict[str, str]:
    """Map package name to version from ``Cargo.lock``."""
    document = tomllib.loads(lock_text)
    packages = document.get("package", [])
    if not isinstance(packages, list):
        raise SbomError("Cargo.lock has no package table")
    return {entry["name"]: entry["version"] for entry in packages}


def dependency_edges(lock_text: str) -> dict[str, list[str]]:
    """Map each package to the names it depends on, dropping version suffixes."""
    document = tomllib.loads(lock_text)
    edges: dict[str, list[str]] = {}
    for entry in document.get("package", []):
        names = []
        for dependency in entry.get("dependencies", []):
            names.append(dependency.split(" ", 1)[0])
        edges[entry["name"]] = sorted(names)
    return edges


def component(name: str, version: str, license_id: str | None) -> dict:
    licenses = []
    if license_id is None or license_id not in SPDX_LICENSES:
        licenses.append({"license": {"name": "UNKNOWN"}})
    else:
        licenses.append({"license": {"id": license_id}})
    return {
        "type": "library",
        "bom-ref": f"pkg:cargo/{name}@{version}",
        "name": name,
        "version": version,
        "purl": f"pkg:cargo/{name}@{version}",
        "licenses": licenses,
    }


def build_sbom(root: Path) -> dict:
    """Build the SBOM for the tree at ``root``."""
    manifest_path = root / WORKSPACE_MANIFEST.relative_to(ROOT)
    lock_path = root / LOCKFILE.relative_to(ROOT)
    manifest = load_toml(manifest_path)
    defaults = workspace_defaults(manifest)
    members = workspace_members(manifest)

    if not lock_path.exists():
        raise SbomError(f"missing {LOCKFILE.relative_to(ROOT)}")

    lock_text = lock_path.read_text(encoding="utf-8")
    versions = cargo_lock_packages(lock_text)
    edges = dependency_edges(lock_text)

    components = [
        {
            "type": "application",
            "bom-ref": "pkg:cargo/curbcut-ai@workspace",
            "name": "curbcut-ai",
            "version": defaults.get("version", "unknown"),
            "purl": f"pkg:cargo/curbcut-ai@{defaults.get('version', 'unknown')}",
            "licenses": [{"license": {"id": defaults["license"]}}]
            if defaults.get("license") in SPDX_LICENSES
            else [{"license": {"name": "UNKNOWN"}}],
        }
    ]
    for name in sorted(versions):
        crate_dir = root / "crates" / name
        license_id = (
            resolve_license(crate_dir, defaults) if crate_dir.is_dir() else None
        )
        components.append(component(name, versions[name], license_id))

    dependencies = [
        {"ref": "pkg:cargo/curbcut-ai@workspace", "dependsOn": []}
    ]
    for name in sorted(versions):
        ref = f"pkg:cargo/{name}@{versions[name]}"
        dependencies.append({"ref": ref, "dependsOn": edges.get(name, [])})

    return {
        "bomFormat": "CycloneDX",
        "specVersion": SPEC_VERSION,
        "version": 1,
        "metadata": {
            "component": components[0],
            "tools": [{"name": TOOL_NAME, "version": defaults.get("version", "unknown")}],
        },
        "components": components,
        "dependencies": dependencies,
    }


def render(sbom: dict) -> str:
    """Serialise with stable key order and a trailing newline."""
    return json.dumps(sbom, indent=2, sort_keys=True) + "\n"


def check_sbom(root: Path, path: Path) -> list[str]:
    """Compare a published SBOM with the tree it claims to describe."""
    errors: list[str] = []
    try:
        recorded = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        return [f"{path.name}: invalid JSON: {error}"]
    if not isinstance(recorded, dict):
        return [f"{path.name}: expected a JSON object"]

    if recorded.get("bomFormat") != "CycloneDX":
        errors.append(f"{path.name}: bomFormat is not CycloneDX")
    if recorded.get("specVersion") != SPEC_VERSION:
        errors.append(
            f"{path.name}: specVersion {recorded.get('specVersion')!r} is not "
            f"{SPEC_VERSION}"
        )

    try:
        expected = build_sbom(root)
    except SbomError as error:
        return [f"{path.name}: cannot rebuild the expected SBOM: {error}"]

    expected_components = {entry["name"]: entry for entry in expected["components"]}
    recorded_components = {
        entry.get("name"): entry for entry in recorded.get("components", [])
    }
    for name in sorted(set(expected_components) - set(recorded_components)):
        errors.append(f"{path.name}: {name} is missing from the published SBOM")
    for name in sorted(set(recorded_components) - set(expected_components)):
        errors.append(f"{path.name}: {name} is not in Cargo.lock")

    for name in sorted(set(expected_components) & set(recorded_components)):
        want = expected_components[name]
        have = recorded_components[name]
        for field in ("version", "purl", "bom-ref"):
            if want[field] != have.get(field):
                errors.append(
                    f"{path.name}: {name} {field} is {have.get(field)!r}, "
                    f"expected {want[field]!r}"
                )
        if want["licenses"] != have.get("licenses"):
            errors.append(f"{path.name}: {name} license differs from the manifest")

    expected_edges = {entry["ref"]: entry["dependsOn"] for entry in expected["dependencies"]}
    recorded_edges = {
        entry.get("ref"): entry.get("dependsOn", [])
        for entry in recorded.get("dependencies", [])
    }
    for ref in sorted(expected_edges):
        if recorded_edges.get(ref) != expected_edges[ref]:
            errors.append(f"{path.name}: dependency edges for {ref} differ")
    return errors


def source_revision(root: Path) -> str | None:
    try:
        result = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    revision = result.stdout.strip()
    return revision or None


def build_attestation(root: Path, target: str) -> dict:
    """Write the shape a release fills in. Nothing here claims to be signed."""
    return {
        "artifact": f"curbcut-{target}",
        "target": target,
        "toolchain": None,
        "source_revision": source_revision(root),
        "built_at_ms": None,
        "sha256": None,
        "signature": None,
        "signing": {
            "scheme": "ed25519",
            "status": "unsigned placeholder; the release job fills sha256 and "
            "signature with the maintainer key",
        },
    }


def _self_test() -> int:
    checks = 0

    def expect(condition: bool, label: str) -> None:
        nonlocal checks
        if not condition:
            print(f"SELF-TEST FAILED: {label}", file=sys.stderr)
            raise SystemExit(1)
        checks += 1

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "crates" / "demo-core").mkdir(parents=True)
        (root / "Cargo.toml").write_text(
            "[workspace]\nresolver = \"2\"\nmembers = [\"crates/*\"]\n\n"
            "[workspace.package]\nversion = \"0.1.0\"\nlicense = \"AGPL-3.0-only\"\n",
            encoding="utf-8",
        )
        (root / "crates" / "demo-core" / "Cargo.toml").write_text(
            "[package]\nname = \"demo-core\"\nversion.workspace = true\n"
            "license.workspace = true\n\n[dependencies]\n",
            encoding="utf-8",
        )
        (root / "Cargo.lock").write_text(
            'version = 4\n\n[[package]]\nname = "demo-core"\nversion = "0.1.0"\n\n'
            '[[package]]\nname = "serde"\nversion = "1.0.200"\n',
            encoding="utf-8",
        )

        first = render(build_sbom(root))
        second = render(build_sbom(root))
        expect(first == second, "SBOM generation is byte-stable")
        document = json.loads(first)
        components = {entry["name"]: entry for entry in document["components"]}
        expect("demo-core" in components, "local crate appears in the SBOM")
        expect("serde" in components, "external crate appears in the SBOM")
        expect(
            components["serde"]["licenses"][0]["license"]["name"] == "UNKNOWN",
            "a license the manifests do not declare stays UNKNOWN",
        )
        expect(
            components["demo-core"]["licenses"][0]["license"]["id"]
            == "AGPL-3.0-only",
            "a declared license maps to its SPDX id",
        )
        expect(
            document["components"][0]["bom-ref"] == "pkg:cargo/curbcut-ai@workspace",
            "the root component leads the list",
        )

        unknown = [
            entry["name"]
            for entry in document["components"]
            if entry["licenses"][0]["license"].get("name") == "UNKNOWN"
        ]
        expect(unknown == ["serde"], "only the undeclared dependency is UNKNOWN")

        # A dependency whose license is declared must resolve.
        (root / "crates" / "serde").mkdir()
        (root / "crates" / "serde" / "Cargo.toml").write_text(
            "[package]\nname = \"serde\"\nversion = \"1.0.200\"\nlicense = \"MIT\"\n",
            encoding="utf-8",
        )
        resolved = json.loads(render(build_sbom(root)))
        serde = {e["name"]: e for e in resolved["components"]}["serde"]
        expect(
            serde["licenses"][0]["license"]["id"] == "MIT",
            "a declared external license resolves",
        )

        # Removing a package from the lock must change the SBOM.
        (root / "Cargo.lock").write_text(
            'version = 4\n\n[[package]]\nname = "demo-core"\nversion = "0.1.0"\n',
            encoding="utf-8",
        )
        smaller = json.loads(render(build_sbom(root)))
        expect(
            [e["name"] for e in smaller["components"]].count("serde") == 0,
            "a removed dependency disappears from the SBOM",
        )

        published = root / "sbom.json"
        published.write_text(render(build_sbom(root)), encoding="utf-8")
        expect(check_sbom(root, published) == [], "a generated SBOM round-trips")

        tampered = json.loads(published.read_text(encoding="utf-8"))
        for entry in tampered["components"]:
            if entry["name"] == "demo-core":
                entry["version"] = "9.9.9"
        published.write_text(json.dumps(tampered), encoding="utf-8")
        expect(
            any("demo-core version" in error for error in check_sbom(root, published)),
            "a rewritten version is reported",
        )

        tampered = json.loads(published.read_text(encoding="utf-8"))
        tampered["components"] = [
            entry for entry in tampered["components"] if entry["name"] != "demo-core"
        ]
        published.write_text(json.dumps(tampered), encoding="utf-8")
        expect(
            any(
                "missing from the published SBOM" in error
                for error in check_sbom(root, published)
            ),
            "a deleted component is reported",
        )

    print(f"self-test: {checks} SBOM checks passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true", help="run built-in checks")
    sub = parser.add_subparsers(dest="command")
    generate = sub.add_parser("generate", help="write an SBOM and attestation")
    generate.add_argument("--output", type=Path, required=True)
    generate.add_argument("--attestation", type=Path)
    generate.add_argument("--target", default="x86_64-unknown-linux-gnu")
    generate.add_argument(
        "--strict", action="store_true", help="fail on UNKNOWN licenses"
    )
    check = sub.add_parser("check", help="verify an SBOM against the tree")
    check.add_argument("--sbom", type=Path, required=True)
    check.add_argument("--strict", action="store_true", help="fail on UNKNOWN licenses")
    args = parser.parse_args()

    if args.self_test:
        return _self_test()

    if args.command == "check":
        errors = check_sbom(ROOT, args.sbom)
        unknown = sorted(
            entry["name"]
            for entry in json.loads(args.sbom.read_text(encoding="utf-8")).get(
                "components", []
            )
            if entry.get("licenses", [{}])[0].get("license", {}).get("name") == "UNKNOWN"
        ) if args.strict else []
        if args.strict and unknown:
            errors.append(
                f"{args.sbom}: {len(unknown)} component(s) have no resolvable "
                "license: " + ", ".join(unknown)
            )
        for message in errors:
            print(f"FAIL: {message}", file=sys.stderr)
        if errors:
            print(f"{len(errors)} SBOM check(s) failed", file=sys.stderr)
            return 1
        print(f"{args.sbom} matches the manifests")
        return 0

    if args.command != "generate":
        parser.print_help()
        return 2

    try:
        sbom = build_sbom(ROOT)
    except SbomError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1

    unknown = sorted(
        entry["name"]
        for entry in sbom["components"]
        if entry["licenses"][0]["license"].get("name") == "UNKNOWN"
    )
    if args.strict and unknown:
        print(
            f"FAIL: {len(unknown)} component(s) have no resolvable license: "
            + ", ".join(unknown),
            file=sys.stderr,
        )
        return 1

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(render(sbom), encoding="utf-8")
    print(f"wrote {args.output} ({len(sbom['components'])} components)")

    if args.attestation:
        args.attestation.parent.mkdir(parents=True, exist_ok=True)
        args.attestation.write_text(
            json.dumps(build_attestation(ROOT, args.target), indent=2, sort_keys=True)
            + "\n",
            encoding="utf-8",
        )
        print(f"wrote {args.attestation} (unsigned placeholder)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
