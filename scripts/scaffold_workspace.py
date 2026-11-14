#!/usr/bin/env python3
"""Generate the 14-crate Cargo workspace skeleton.

Run from the repository root:
    python3 scripts/scaffold_workspace.py

Idempotent for existing crate files: it overwrites manifests, lib.rs, and READMEs,
but never touches source files it did not generate.
"""

from __future__ import annotations

import argparse
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

WORKSPACE_TOML = """\
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "AGPL-3.0-only"
repository = "https://github.com/10xdev4u-alt/curbcut-ai"
rust-version = "1.85"

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[profile.release]
lto = "thin"
strip = true
"""

# name, description, dependencies
CRATES = [
    ("curbcut-core", "Event bus, config, errors, capability tokens, orchestration.", []),
    ("curbcut-capture", "Screen capture adapters and the zeroizing frame ring buffer.", ["curbcut-core"]),
    ("curbcut-a11y", "Accessibility tree adapters and the unified element model.", ["curbcut-core"]),
    ("curbcut-vision", "ONNX inference, detection, OCR, layout, and motion scheduling.", ["curbcut-core", "curbcut-capture"]),
    ("curbcut-understand", "Classification, chunking, simplification, and reading level.", ["curbcut-core"]),
    ("curbcut-adapt", "Profiles, rules engine, layout solver, and plan cache.", ["curbcut-core"]),
    ("curbcut-render", "Overlay plan types, hit-testing, and rendering contracts.", ["curbcut-core", "curbcut-adapt"]),
    ("curbcut-tts", "TTS abstraction, audio focus, and voice command grammar.", ["curbcut-core"]),
    ("curbcut-input", "Global shortcuts, hover/click routing, and switch access.", ["curbcut-core"]),
    ("curbcut-store", "Storage schema, migrations, repositories, and key handling.", ["curbcut-core"]),
    ("curbcut-telemetry", "Opt-in, counts-only telemetry and redaction.", ["curbcut-core"]),
    ("curbcut-cli", "Headless control, benchmark, and redacted debug dump.", ["curbcut-core"]),
    ("curbcut-app", "Tauri app shell and frontend bridge (stub until the toolchain lands).", ["curbcut-core"]),
    ("curbcut-macros", "Typed event and provenance macros.", []),
]


def crate_toml(name: str, description: str, deps: list[str], features: list[str] | None = None) -> str:
    lines = [
        "[package]",
        'name = "%s"' % name,
        "version.workspace = true",
        "edition.workspace = true",
        "license.workspace = true",
        "repository.workspace = true",
        'description = "%s"' % description,
        "publish = false",
        "",
        "[lints]",
        "workspace = true",
        "",
        "[dependencies]",
    ]
    for dep in deps:
        lines.append('%s = { path = "../%s" }' % (dep, dep))
    if features:
        lines += ["", "[features]"]
        for feature in features:
            lines.append("%s = []" % feature)
    return "\n".join(lines) + "\n"


def lib_rs(name: str, description: str, extra: str = "") -> str:
    return """\
//! %(desc)s
//!
//! Skeleton crate. The public API is added incrementally; see
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "%(name)s";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
%(extra)s

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "%(name)s");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
""" % {"name": name, "desc": description, "extra": extra}


CRATE_STATUS = {
    "curbcut-cli": (
        "The headless CLI is implemented: status, redacted dump, benchmark, and\n"
        "panic commands, with stable exit codes. See\n"
        "[`../../docs/CLI.md`](../../docs/CLI.md)."
    ),
    "curbcut-understand": (
        "The deterministic rule layer is implemented: versioned rules, role\n"
        "refinement, chunking, reading-level comparison, and distraction scoring.\n"
        "See [`../../docs/RULES.md`](../../docs/RULES.md)."
    ),
}


def readme(name: str, description: str, status: str | None = None) -> str:
    if status is None:
        status = (
            "Skeleton. The API is added incrementally; see\n"
            "[`../../backlog/EPICS.md`](../../backlog/EPICS.md) and\n"
            "[`../../backlog/ISSUES.md`](../../backlog/ISSUES.md)."
        )
    return """\
# %(name)s

%(desc)s

## Status

%(status)s

## Build and test

```bash
cargo test -p %(name)s
cargo clippy -p %(name)s --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
""" % {"name": name, "desc": description, "status": status}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--force", action="store_true", help="overwrite existing src/lib.rs")
    args = parser.parse_args()

    (ROOT / "Cargo.toml").write_text(WORKSPACE_TOML, encoding="utf-8")
    crates = ROOT / "crates"
    crates.mkdir(exist_ok=True)
    features = {"curbcut-app": ["tauri"]}
    for name, desc, deps in CRATES:
        d = crates / name
        (d / "src").mkdir(parents=True, exist_ok=True)
        (d / "Cargo.toml").write_text(
            crate_toml(name, desc, deps, features.get(name, [])), encoding="utf-8"
        )
        lib = d / "src" / "lib.rs"
        extra = (
            "\n/// Whether the Tauri feature is enabled in this build.\n"
            "pub fn tauri_enabled() -> bool {\n    cfg!(feature = \"tauri\")\n}\n"
            if name == "curbcut-app"
            else ""
        )
        if lib.exists() and not args.force:
            print("kept", lib)
        else:
            lib.write_text(lib_rs(name, desc, extra), encoding="utf-8")
        (d / "README.md").write_text(readme(name, desc, CRATE_STATUS.get(name)), encoding="utf-8")
        print("scaffolded", name)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
