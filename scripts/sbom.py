#!/usr/bin/env python3
"""Generate an SBOM and a release attestation skeleton from the manifests.

CC-119 asks for a published SBOM listing every dependency and its license, plus
a provenance attestation per artifact. This script derives both from files that
are already in the repository, so it needs no network and no third-party
packages:

* ``Cargo.lock`` names every package in the build graph, including transitive
  ones, with the exact versions resolved;
* each crate's ``Cargo.toml`` carries its declared license, and
  workspace-inherited values are read from the workspace root;
* anything outside the workspace must have its license recorded in
  ``supply-chain/registry-licenses.json``.

The SBOM is a CycloneDX-shaped document. It deliberately carries **no**
timestamp and **no** random serial number, so two runs over the same tree
produce the same bytes; that property is part of the point, and ``--self-test``
asserts it. Every dependency edge is a full ``bom-ref``, so the published graph
resolves to components in the same document.

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
REGISTRY_LICENSES = "supply-chain/registry-licenses.json"

SPEC_VERSION = "1.6"
TOOL_NAME = "curbcut-sbom"
ROOT_REF = "pkg:cargo/curbcut-ai@workspace"

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


def ref(name: str, version: str) -> str:
    return f"pkg:cargo/{name}@{version}"


def parse_lock(text: str) -> list[dict]:
    """Return one entry per locked package, with its dependency name/version pairs."""
    document = tomllib.loads(text)
    packages = document.get("package", [])
    if not isinstance(packages, list) or not packages:
        raise SbomError("Cargo.lock has no package table")
    parsed: list[dict] = []
    for entry in packages:
        dependencies = []
        for dependency in entry.get("dependencies", []):
            parts = dependency.split(" ", 1)
            dependencies.append(
                (parts[0], parts[1] if len(parts) > 1 else None)
            )
        parsed.append(
            {"name": entry["name"], "version": entry["version"], "deps": dependencies}
        )
    return parsed


def resolve_dependency(
    name: str, version_hint: str | None, locked: set[tuple[str, str]]
) -> str:
    """Turn a lockfile dependency string into a single resolvable bom-ref."""
    if version_hint is not None and (name, version_hint) in locked:
        return ref(name, version_hint)
    matches = sorted(version for pkg, version in locked if pkg == name)
    if len(matches) == 1:
        return ref(name, matches[0])
    raise SbomError(
        f"dependency {name!r} resolves to {len(matches)} locked versions; "
        "the lockfile is ambiguous"
    )


def load_registry_licenses(root: Path) -> dict[str, str]:
    """Read the recorded licenses for packages that live outside the workspace."""
    path = root / REGISTRY_LICENSES
    if not path.exists():
        return {}
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise SbomError(f"{REGISTRY_LICENSES}: invalid JSON: {error}") from error
    if not isinstance(document, dict):
        raise SbomError(f"{REGISTRY_LICENSES}: expected an object")
    return {str(key): str(value) for key, value in document.items()}


def registry_license(name: str, version: str, registry: dict[str, str]) -> str | None:
    return registry.get(f"{name}@{version}") or registry.get(name)


def resolve_crate_license(crate_dir: Path, defaults: dict) -> str | None:
    """Return the declared license for one workspace crate, or ``None``."""
    package = load_toml(crate_dir / "Cargo.toml").get("package", {})
    value = package.get("license")
    if isinstance(value, dict):
        if value.get("workspace"):
            return defaults.get("license")
        value = value.get("file")  # `license = { file = ... }`
        return f"file:{value}" if value else None
    if value is None:
        value = defaults.get("license")
    return value


def component(name: str, version: str, license_id: str | None) -> dict:
    licenses = (
        [{"license": {"id": license_id}}]
        if license_id in SPDX_LICENSES
        else [{"license": {"name": "UNKNOWN"}}]
    )
    return {
        "type": "library",
        "bom-ref": ref(name, version),
        "name": name,
        "version": version,
        "purl": ref(name, version),
        "licenses": licenses,
    }


def build_sbom(root: Path) -> dict:
    """Build the SBOM for the tree at ``root``."""
    manifest_path = root / WORKSPACE_MANIFEST.relative_to(ROOT)
    lock_path = root / LOCKFILE.relative_to(ROOT)
    if not manifest_path.exists():
        raise SbomError(f"missing {WORKSPACE_MANIFEST.relative_to(ROOT)}")
    if not lock_path.exists():
        raise SbomError(f"missing {LOCKFILE.relative_to(ROOT)}")

    manifest = load_toml(manifest_path)
    defaults = manifest.get("workspace", {}).get("package", {})
    packages = parse_lock(lock_path.read_text(encoding="utf-8"))
    locked = {(entry["name"], entry["version"]) for entry in packages}
    registry = load_registry_licenses(root)

    root_component = {
        "type": "application",
        "bom-ref": ROOT_REF,
        "name": "curbcut-ai",
        "version": defaults.get("version", "unknown"),
        "purl": ROOT_REF,
        "licenses": [{"license": {"id": defaults["license"]}}]
        if defaults.get("license") in SPDX_LICENSES
        else [{"license": {"name": "UNKNOWN"}}],
    }

    components = []
    for entry in sorted(packages, key=lambda item: (item["name"], item["version"])):
        crate_dir = root / "crates" / entry["name"]
        if crate_dir.is_dir():
            license_id = resolve_crate_license(crate_dir, defaults)
        else:
            license_id = registry_license(entry["name"], entry["version"], registry)
        components.append(component(entry["name"], entry["version"], license_id))

    dependencies = []
    for entry in sorted(packages, key=lambda item: (item["name"], item["version"])):
        source = ref(entry["name"], entry["version"])
        targets = sorted(
            {
                resolve_dependency(name, version, locked)
                for name, version in entry["deps"]
            }
        )
        dependencies.append({"ref": source, "dependsOn": targets})

    return {
        "bomFormat": "CycloneDX",
        "specVersion": SPEC_VERSION,
        "version": 1,
        "metadata": {
            "component": root_component,
            "tools": [
                {"name": TOOL_NAME, "version": defaults.get("version", "unknown")}
            ],
        },
        "components": components,
        "dependencies": dependencies,
    }


def render(sbom: dict) -> str:
    """Serialise with stable key order and a trailing newline."""
    return json.dumps(sbom, indent=2, sort_keys=True) + "\n"


def unknown_components(sbom: dict) -> list[str]:
    return sorted(
        entry["name"]
        for entry in sbom["components"]
        if entry["licenses"][0]["license"].get("name") == "UNKNOWN"
    )


def check_sbom(root: Path, path: Path) -> list[str]:
    """Compare a published SBOM with the tree it claims to describe."""
    errors: list[str] = []
    try:
        recorded = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        return [f"{path.name}: invalid JSON: {error}"]
    if not isinstance(recorded, dict):
        return [f"{path.name}: expected a JSON object"]

    for field, wanted in (
        ("bomFormat", "CycloneDX"),
        ("specVersion", SPEC_VERSION),
        ("version", 1),
    ):
        if recorded.get(field) != wanted:
            errors.append(
                f"{path.name}: {field} is {recorded.get(field)!r}, expected {wanted!r}"
            )

    try:
        expected = build_sbom(root)
    except SbomError as error:
        return [f"{path.name}: cannot rebuild the expected SBOM: {error}"]

    if recorded.get("metadata", {}).get("component") != expected["metadata"]["component"]:
        errors.append(f"{path.name}: metadata.component differs from the workspace")

    expected_components = {entry["bom-ref"]: entry for entry in expected["components"]}
    recorded_components = {
        entry.get("bom-ref"): entry for entry in recorded.get("components", [])
    }
    for bom_ref in sorted(set(expected_components) - set(recorded_components)):
        errors.append(f"{path.name}: {bom_ref} is missing from the published SBOM")
    for bom_ref in sorted(set(recorded_components) - set(expected_components)):
        errors.append(f"{path.name}: {bom_ref} is not in Cargo.lock")

    for bom_ref in sorted(set(expected_components) & set(recorded_components)):
        want = expected_components[bom_ref]
        have = recorded_components[bom_ref]
        for field in ("type", "name", "version", "purl"):
            if want[field] != have.get(field):
                errors.append(
                    f"{path.name}: {bom_ref} {field} is {have.get(field)!r}, "
                    f"expected {want[field]!r}"
                )
        if want["licenses"] != have.get("licenses"):
            errors.append(f"{path.name}: {bom_ref} license differs from the manifest")

    expected_edges = {entry["ref"]: entry["dependsOn"] for entry in expected["dependencies"]}
    recorded_edges = {
        entry.get("ref"): entry.get("dependsOn", [])
        for entry in recorded.get("dependencies", [])
    }
    for bom_ref in sorted(set(expected_edges) - set(recorded_edges)):
        errors.append(f"{path.name}: no dependency entry for {bom_ref}")
    for bom_ref in sorted(set(recorded_edges) - set(expected_edges)):
        errors.append(f"{path.name}: unexpected dependency entry for {bom_ref}")
    for bom_ref in sorted(set(expected_edges) & set(recorded_edges)):
        if expected_edges[bom_ref] != recorded_edges[bom_ref]:
            errors.append(f"{path.name}: dependency edges for {bom_ref} differ")
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


def _write_workspace(root: Path, *, external: dict[str, str] | None = None) -> None:
    (root / "crates" / "demo-core").mkdir(parents=True, exist_ok=True)
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
        'version = 4\n\n[[package]]\nname = "demo-core"\nversion = "0.1.0"\n',
        encoding="utf-8",
    )
    for name, version in (external or {}).items():
        entry = f'[[package]]\nname = "{name}"\nversion = "{version}"\n'
        current = (root / "Cargo.lock").read_text(encoding="utf-8")
        (root / "Cargo.lock").write_text(current + "\n" + entry, encoding="utf-8")


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
        _write_workspace(root, external={"serde": "1.0.200"})
        expect(
            unknown_components(build_sbom(root)) == ["serde"],
            "a package with no recorded license stays UNKNOWN",
        )

        first = render(build_sbom(root))
        second = render(build_sbom(root))
        expect(first == second, "SBOM generation is byte-stable")

        document = json.loads(first)
        expect(
            [entry["bom-ref"] for entry in document["components"]].count(ROOT_REF) == 0,
            "the workspace root appears once, in metadata",
        )
        expect(
            document["metadata"]["component"]["bom-ref"] == ROOT_REF,
            "the root bom-ref is unique",
        )
        refs = {entry["bom-ref"] for entry in document["components"]}
        for entry in document["dependencies"]:
            for target in entry["dependsOn"]:
                expect(target in refs, f"edge {target} resolves to a component")

        (root / "supply-chain").mkdir()
        (root / REGISTRY_LICENSES).write_text('{"serde": "MIT"}\n', encoding="utf-8")
        expect(
            unknown_components(build_sbom(root)) == [],
            "a recorded registry license resolves",
        )

        published = root / "sbom.json"
        published.write_text(render(build_sbom(root)), encoding="utf-8")
        expect(check_sbom(root, published) == [], "a generated SBOM round-trips")

        tampered = json.loads(published.read_text(encoding="utf-8"))
        tampered["components"][0]["version"] = "9.9.9"
        published.write_text(json.dumps(tampered), encoding="utf-8")
        expect(
            any("version is" in error for error in check_sbom(root, published)),
            "a rewritten version is reported",
        )

        tampered = json.loads(published.read_text(encoding="utf-8"))
        tampered["components"] = tampered["components"][1:]
        published.write_text(json.dumps(tampered), encoding="utf-8")
        expect(
            any(
                "missing from the published SBOM" in error
                for error in check_sbom(root, published)
            ),
            "a deleted component is reported",
        )

        tampered = json.loads(published.read_text(encoding="utf-8"))
        tampered["metadata"]["component"]["licenses"] = [
            {"license": {"id": "MIT"}}
        ]
        published.write_text(json.dumps(tampered), encoding="utf-8")
        expect(
            any("metadata.component" in error for error in check_sbom(root, published)),
            "a swapped root license is reported",
        )

        tampered = json.loads(published.read_text(encoding="utf-8"))
        tampered["dependencies"] = tampered["dependencies"][1:]
        published.write_text(json.dumps(tampered), encoding="utf-8")
        expect(
            any(
                "no dependency entry" in error for error in check_sbom(root, published)
            ),
            "a dropped dependency entry is reported",
        )

        tampered = json.loads(published.read_text(encoding="utf-8"))
        tampered["dependencies"][0]["dependsOn"] = ["pkg:cargo/not-a-real-crate@0.1.0"]
        published.write_text(json.dumps(tampered), encoding="utf-8")
        expect(
            any("edges for" in error for error in check_sbom(root, published)),
            "a dangling dependency edge is reported",
        )

        # Two versions of one crate must both survive, keyed by bom-ref.
        _write_workspace(root, external={})
        lock = (root / "Cargo.lock").read_text(encoding="utf-8")
        lock += '\n[[package]]\nname = "demo-core"\nversion = "0.2.0"\n'
        (root / "Cargo.lock").write_text(lock, encoding="utf-8")
        duplicated = build_sbom(root)
        names = [entry["name"] for entry in duplicated["components"]]
        expect(names.count("demo-core") == 2, "both locked versions are listed")
        refs = [entry["bom-ref"] for entry in duplicated["components"]]
        expect(len(refs) == len(set(refs)), "bom-refs stay unique across versions")

        ambiguous = parse_lock(
            'version = 4\n\n[[package]]\nname = "demo-core"\nversion = "0.1.0"\n\n'
            '[[package]]\nname = "demo-core"\nversion = "0.2.0"\n\n'
            '[[package]]\nname = "app"\nversion = "0.1.0"\ndependencies = '
            '["demo-core"]\n'
        )
        locked = {(entry["name"], entry["version"]) for entry in ambiguous}
        try:
            resolve_dependency("demo-core", None, locked)
        except SbomError:
            checks += 1
        else:
            print("SELF-TEST FAILED: an ambiguous dependency was accepted", file=sys.stderr)
            raise SystemExit(1)

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
        if args.strict:
            try:
                document = json.loads(args.sbom.read_text(encoding="utf-8"))
            except json.JSONDecodeError as error:
                return print(f"FAIL: {args.sbom}: invalid JSON: {error}", file=sys.stderr) or 1
            unknown = unknown_components(document)
            if unknown:
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

    if args.strict:
        unknown = unknown_components(sbom)
        if unknown:
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
