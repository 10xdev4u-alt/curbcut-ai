#!/usr/bin/env python3
"""Generate the 14-crate Cargo workspace skeleton.

Run from the repository root:
    python3 scripts/scaffold_workspace.py

Idempotent for existing crate files: it overwrites manifests, lib.rs, and READMEs,
but never touches source files it did not generate.
"""

from __future__ import annotations

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


def crate_toml(name: str, description: str, deps: list[str]) -> str:
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
        "[dependencies]",
    ]
    for dep in deps:
        lines.append('%s = { path = "../%s" }' % (dep, dep))
    return "\n".join(lines) + "\n"


def lib_rs(name: str, description: str) -> str:
    return """\
//! %(desc)s
//!
//! Skeleton crate. The public API is added incrementally; see
//! [`../../backlog/EPICS.md`](../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "%(name)s";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "%(name)s");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
""" % {"name": name, "desc": description}


def readme(name: str, description: str) -> str:
    return """\
# %(name)s

%(desc)s

## Status

Skeleton. The API is added incrementally; see
[`../backlog/EPICS.md`](../backlog/EPICS.md) and
[`../backlog/ISSUES.md`](../backlog/ISSUES.md).

## Build and test

```bash
cargo test -p %(name)s
cargo clippy -p %(name)s --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
""" % {"name": name, "desc": description}


def main() -> int:
    (ROOT / "Cargo.toml").write_text(WORKSPACE_TOML, encoding="utf-8")
    crates = ROOT / "crates"
    crates.mkdir(exist_ok=True)
    for name, desc, deps in CRATES:
        d = crates / name
        (d / "src").mkdir(parents=True, exist_ok=True)
        (d / "Cargo.toml").write_text(crate_toml(name, desc, deps), encoding="utf-8")
        (d / "src" / "lib.rs").write_text(lib_rs(name, desc), encoding="utf-8")
        (d / "README.md").write_text(readme(name, desc), encoding="utf-8")
        print("scaffolded", name)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
