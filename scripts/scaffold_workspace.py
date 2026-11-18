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
    ("curbcut-adapt", "Profiles, rules engine, and layout solver.", ["curbcut-core"]),
    ("curbcut-render", "Overlay plan types, hit-testing, and rendering contracts.", ["curbcut-core", "curbcut-adapt"]),
    ("curbcut-tts", "TTS abstraction, audio focus, and voice command grammar.", ["curbcut-core"]),
    ("curbcut-input", "Global shortcuts, hover/click routing, and switch access.", ["curbcut-core"]),
    ("curbcut-store", "Storage schema, migrations, repositories, key handling, and the plan cache.", ["curbcut-core"]),
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
    "curbcut-a11y": (
        "The platform role and state mapping is implemented, and the a11y +\n"
        "vision merge: candidates are matched by IoU and role compatibility,\n"
        "taken greedily in deterministic order, and each merged property keeps\n"
        "its provenance. A disagreement becomes a content-free conflict, an\n"
        "unmatched a11y element stays semantic-only, and an unmatched vision\n"
        "detection becomes a vision-only node, and stability tracking assigns\n"
        "StableIds across frames with median scroll compensation and a\n"
        "Hungarian assignment. See\n"
        "[`../../docs/MERGE.md`](../../docs/MERGE.md) and\n"
        "[`../../docs/STABILITY.md`](../../docs/STABILITY.md)."
    ),
    "curbcut-adapt": (
        "The profile graph and layout solvers are implemented: a versioned\n"
        "settings graph with a baseline, atomic multi-setting changes,\n"
        "live-preview sinks, and a 40-key setting registry; a priority-based\n"
        "tile packer plus a no-clip reflow; an icon-grid layout with a\n"
        "label policy and search; a dimming planner that covers the screen\n"
        "except the focused control and except-regions; confirmation with\n"
        "undo for destructive actions; and a provenance panel whose entries\n"
        "link every change to its rule and to a log-safe original. See\n"
        "[`../../docs/PROFILE_GRAPH.md`](../../docs/PROFILE_GRAPH.md),\n"
        "[`../../docs/LAYOUT.md`](../../docs/LAYOUT.md),\n"
        "[`../../docs/GRID.md`](../../docs/GRID.md),\n"
        "[`../../docs/DIMMING.md`](../../docs/DIMMING.md),\n"
        "[`../../docs/CONFIRM.md`](../../docs/CONFIRM.md),\n"
        "[`../../docs/PROVENANCE.md`](../../docs/PROVENANCE.md),\n"
        "[`../../docs/STOP_MOTION.md`](../../docs/STOP_MOTION.md), and\n"
        "[`../../docs/ONE_STEP.md`](../../docs/ONE_STEP.md)."
    ),
    "curbcut-cli": (
        "The headless CLI is implemented: status, redacted dump, benchmark, and\n"
        "panic commands, with stable exit codes. See\n"
        "[`../../docs/CLI.md`](../../docs/CLI.md)."
    ),
    "curbcut-vision": (
        "The vision pipeline is implemented: Detector, Ocr, and IconCaptioner\n"
        "traits, a deterministic motion-aware scheduler, and VisionPipeline.\n"
        "See [`../../docs/VISION.md`](../../docs/VISION.md)."
    ),
    "curbcut-understand": (
        "The deterministic rule layer is implemented: versioned rules, role\n"
        "refinement, chunking, reading-level comparison, and distraction scoring.\n"
        "Reading level scores Flesch, Flesch-Kincaid, SMOG, and ARI, flags\n"
        "non-Latin scripts as unsupported, and caches scores by content hash.\n"
        "The jargon dictionary carries a source and locale per entry, supports\n"
        "user overrides, and expands acronyms while flagging unknown ones.\n"
        "Numbers, dates, and measurements are spoken: money in hundreds,\n"
        "ambiguous dates rejected, and a locale's unit system honoured.\n"
        "The sensitive-content guard refuses to auto-simplify financial,\n"
        "medical, and legal text and always returns the original. A versioned\n"
        "destructive-action catalog returns a plain preview and requires\n"
        "confirmation before anything is performed. The meaning-preservation\n"
        "checker rejects a rewrite that drops a named entity, a number, a\n"
        "negation, or a comparative, and logs rejections as counts only. Text\n"
        "chunking splits plain text into headings, list items, tables, sentences,\n"
        "and clauses as half-open byte ranges, keeps a table whole, and never\n"
        "splits mid-word. Interruption detection reports the transient overlays\n"
        "in a merged graph — toast, banner, popup, cookie notice, or paywall —\n"
        "with a rectangle and a dismissibility flag, and reads labels in memory\n"
        "only. See\n"
        "[`../../docs/RULES.md`](../../docs/RULES.md),\n"
        "[`../../docs/READING_LEVEL.md`](../../docs/READING_LEVEL.md),\n"
        "[`../../docs/JARGON.md`](../../docs/JARGON.md),\n"
        "[`../../docs/TEXT_CHUNKING.md`](../../docs/TEXT_CHUNKING.md),\n"
        "[`../../docs/INTERRUPTIONS.md`](../../docs/INTERRUPTIONS.md),\n"
        "[`../../docs/NORMALIZE.md`](../../docs/NORMALIZE.md),\n"
        "[`../../docs/SENSITIVE_CONTENT.md`](../../docs/SENSITIVE_CONTENT.md), and\n"
        "[`../../docs/DESTRUCTIVE_ACTIONS.md`](../../docs/DESTRUCTIVE_ACTIONS.md), and\n"
        "[`../../docs/MEANING.md`](../../docs/MEANING.md)."
    ),
    "curbcut-store": (
        "The plan cache is implemented: a bounded LRU cache of overlay plans\n"
        "keyed by app, window, and content hash, and opt-in per-app memory for\n"
        "one remembered choice per app. See\n"
        "[`../../docs/PLAN_CACHE.md`](../../docs/PLAN_CACHE.md)."
    ),
    "curbcut-input": (
        "The global shortcut catalog is implemented: `ShortcutAction` lists\n"
        "twelve actions, `ShortcutCatalog` holds the bindings, detects conflicts,\n"
        "and rebinds with a taken-chord check. The pointer tracker is\n"
        "implemented: `HoverTracker` classifies movement from samples so a hover\n"
        "is dwell, not presence. Click-through routing is implemented:\n"
        "`ClickRouter` consumes or hands off each click, never both, and a\n"
        "handoff owes focus until it returns. The keyboard focus order is\n"
        "implemented: `FocusOrder` visits enabled controls in order, wraps, keeps\n"
        "a trapping control until `escape`, lists every shortcut for the UI, and\n"
        "moves spatially with the arrow keys. Switch access is implemented:\n"
        "`SwitchAccess` drives the overlay from one switch with auto scan, step\n"
        "scan, and a configurable dwell. See\n"
        "[`../../docs/SHORTCUTS.md`](../../docs/SHORTCUTS.md),\n"
        "[`../../docs/HOVER.md`](../../docs/HOVER.md),\n"
        "[`../../docs/CLICKS.md`](../../docs/CLICKS.md),\n"
        "[`../../docs/FOCUS_ORDER.md`](../../docs/FOCUS_ORDER.md), and\n"
        "[`../../docs/SWITCH_ACCESS.md`](../../docs/SWITCH_ACCESS.md)."
    ),
    "curbcut-render": (
        "The focus surfaces are implemented: `ReadingRuler` highlights one to\n"
        "three text lines and `FocusLens` magnifies a region with a pointer lead\n"
        "and a jitter dead zone; both are disabled under reduced motion. The\n"
        "animation policy caps every animation at 150 ms, applies an essential\n"
        "change instantly under reduced motion, refuses decoration, and never\n"
        "allows a vestibular trigger. Large-target mode scales every target to\n"
        "a 60 px minimum, translates rather than crops at the maximum scale, and\n"
        "keeps targets at least 8 px apart. See\n"
        "[`../../docs/FOCUS.md`](../../docs/FOCUS.md),\n"
        "[`../../docs/ANIMATION.md`](../../docs/ANIMATION.md), and\n"
        "[`../../docs/LARGE_TARGETS.md`](../../docs/LARGE_TARGETS.md)."
    ),
    "curbcut-tts": (
        "The speech abstraction is implemented: `TtsEngine` is the adapter\n"
        "trait, `ScriptEngine` is the always-available local fallback, and\n"
        "`FallbackChain` skips engines that are unavailable, over the start\n"
        "budget, or on the network. `VoiceCatalog` gates downloads behind an\n"
        "opt-in and `AudioFocus` is a single-holder state machine. The voice\n"
        "command grammar matches the longest catalog phrase on a word boundary,\n"
        "renders the \"what can I say?\" panel, and `PushToTalk` is a hold-or-\n"
        "toggle microphone state machine. See\n"
        "[`../../docs/SPEECH.md`](../../docs/SPEECH.md) and\n"
        "[`../../docs/VOICE_COMMANDS.md`](../../docs/VOICE_COMMANDS.md)."
    ),
    "curbcut-telemetry": (
        "The counts-only telemetry schema is implemented: `MetricKey` is a\n"
        "16-entry allowlist, `Snapshot` serializes keys and numbers and parses\n"
        "the same format back while rejecting an unknown key, and\n"
        "`TelemetryExporter` renders an export only after an opt-in and\n"
        "clears the counts on opt-out. See\n"
        "[`../../docs/TELEMETRY.md`](../../docs/TELEMETRY.md)."
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
