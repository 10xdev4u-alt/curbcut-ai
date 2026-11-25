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
    ("curbcut-core", "Event bus, config, errors, capability tokens, orchestration, zeroizing buffers, golden plan snapshots, SHA-256/HMAC, the seeded property-test harness, and failure injection.", []),
    ("curbcut-capture", "Screen capture adapters and the zeroizing frame ring buffer.", ["curbcut-core"]),
    ("curbcut-a11y", "Accessibility tree adapters and the unified element model.", ["curbcut-core"]),
    ("curbcut-vision", "ONNX inference, detection, OCR, layout, and motion scheduling.", ["curbcut-core", "curbcut-capture"]),
    ("curbcut-understand", "Classification, chunking, simplification, summarization, and reading level.", ["curbcut-core"]),
    ("curbcut-adapt", "Profiles, rules engine, layout solver, typography, and the explanation report.", ["curbcut-core", "curbcut-understand"]),
    ("curbcut-render", "Overlay plan types, hit-testing, and rendering contracts.", ["curbcut-core", "curbcut-adapt"]),
    ("curbcut-tts", "TTS abstraction, audio focus, voice command grammar, and reading position.", ["curbcut-core"]),
    ("curbcut-input", "Global shortcuts, hover/click routing, and switch access.", ["curbcut-core"]),
    ("curbcut-store", "Storage schema, migrations, repositories, key handling, the plan cache, the sealed understanding cache, per-app layout memory, and learned simplifications.", ["curbcut-core"]),
    ("curbcut-telemetry", "Opt-in, counts-only telemetry and redaction.", ["curbcut-core"]),
    ("curbcut-cli", "Headless control, benchmark, redacted debug dump, and the accessibility mapping/VPAT generator.", ["curbcut-core", "curbcut-store"]),
    ("curbcut-app", "Tauri app shell and frontend bridge (stub until the toolchain lands).", ["curbcut-core"]),
    ("curbcut-macros", "Typed event and provenance macros.", []),
]


def crate_toml(
    name: str,
    description: str,
    deps: list[str],
    features: list[str] | None = None,
    default_features: list[str] | None = None,
) -> str:
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
    if features or default_features is not None:
        lines += ["", "[features]"]
        if default_features is not None:
            lines.append(
                "default = [%s]" % ", ".join('"%s"' % feature for feature in default_features)
            )
        for feature in features or []:
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
    "curbcut-core": (
        "Capability tokens, the typed command registry, versioned config, the\n"
        "event bus, strongly typed ids, the redaction backstop, and the shared\n"
        "value types are implemented. The network policy is implemented and\n"
        "opt-in: `net` is the compiled gate (`default = []`), so a plain build\n"
        "has no optional transport module at all, and\n"
        "`net::transport_available` reports which build is running.\n"
        "`net::KillSwitch` is the runtime gate and starts engaged, and\n"
        "`net::NetworkPolicy` defaults to offline. `KillSwitch::from_config`\n"
        "can release it only when the root `update_check` preference is true;\n"
        "no shipped binary calls that path yet. Only `net::update::dial`\n"
        "exists without the feature, it takes the feature it dials and refuses\n"
        "anything but `UpdateCheck`, and it still consults the switch. The\n"
        "`socket-guard` CI job runs `scripts/socket_guard.py` over the\n"
        "`net_probe` example in both builds. The `secure` module adds the\n"
        "zeroizing `SecureBuffer` and `SecureText` types with a shared\n"
        "`GUARD_BYTE` and a thread-local drop counter. The `snapshot` module\n"
        "renders a content-free, deterministic `OverlayPlan` for the golden\n"
        "tests. The `hash` module is a dependency-free SHA-256 and HMAC-SHA-256,\n"
        "checked against the published test vectors. The `prop` module is the\n"
        "seeded property-test harness every crate uses: a splitmix64 generator,\n"
        "a runner that reports the seed and case index on a break, and a\n"
        "wall-clock budget for fuzz runs. The `chaos` module is the failure\n"
        "injection harness: scripted faults, a panic guard that drops the\n"
        "payload, and a report that fails when a fault is unannounced or an\n"
        "announcement does not recover. See\n"
        "[`../../docs/CHAOS_TESTS.md`](../../docs/CHAOS_TESTS.md),\n"
        "[`../../docs/PROPERTY_TESTS.md`](../../docs/PROPERTY_TESTS.md),\n"
        "[`../../docs/NETWORK_POLICY.md`](../../docs/NETWORK_POLICY.md),\n"
        "[`../../docs/SECURE_BUFFERS.md`](../../docs/SECURE_BUFFERS.md),\n"
        "[`../../docs/GOLDEN_TESTS.md`](../../docs/GOLDEN_TESTS.md), and\n"
        "[`../../docs/MODEL_GUARD.md`](../../docs/MODEL_GUARD.md)."
    ),
    "curbcut-a11y": (
        "The platform role and state mapping is implemented, and the a11y +\n"
        "vision merge: candidates are matched by IoU and role compatibility,\n"
        "taken greedily in deterministic order, and each merged property keeps\n"
        "its provenance. A disagreement becomes a content-free conflict, an\n"
        "unmatched a11y element stays semantic-only, and an unmatched vision\n"
        "detection becomes a vision-only node, and stability tracking assigns\n"
        "StableIds across frames with median scroll compensation and a\n"
        "Hungarian assignment. The write guard makes `WriteGuard::request`\n"
        "the only path to another app: it requires a valid `WriteA11y`\n"
        "token, denies and skips the sink otherwise, and records every\n"
        "attempt. The differential test measures the reading order and the\n"
        "mapped roles against a recorded screen-reader baseline and publishes\n"
        "the discrepancy count, so a merge change lowers it or fails. See\n"
        "[`../../docs/MERGE.md`](../../docs/MERGE.md),\n"
        "[`../../docs/STABILITY.md`](../../docs/STABILITY.md),\n"
        "[`../../docs/WRITE_GUARD.md`](../../docs/WRITE_GUARD.md), and\n"
        "[`../../docs/DIFFERENTIAL_TESTS.md`](../../docs/DIFFERENTIAL_TESTS.md)."
    ),
    "curbcut-adapt": (
        "The forward-chaining rules engine is implemented: content-free facts\n"
        "derived from the graph and the media list, a four-rule catalog evaluated\n"
        "in catalog order, priority conflict resolution that records every\n"
        "superseded offer with its winner, and a per-rule toggle. The profile\n"
        "graph and layout solvers are implemented: a versioned\n"
        "settings graph with a baseline, atomic multi-setting changes,\n"
        "live-preview sinks, and a 40-key setting registry; a priority-based\n"
        "tile packer plus a no-clip reflow; an icon-grid layout with a\n"
        "label policy and search; a dimming planner that covers the screen\n"
        "except the focused control and except-regions; confirmation with\n"
        "undo for destructive actions; and a provenance panel whose entries\n"
        "link every change to its rule and to a log-safe original. The\n"
        "forward-chaining rules engine derives content-free facts from a merged\n"
        "graph and the media list, evaluates four built-in rules in catalog\n"
        "order, resolves a conflict by priority, and records every superseded\n"
        "offer with its winner; each rule has a profile flag and a toggle. The\n"
        "interruption inbox files detected interruptions without auto-dismissing\n"
        "them, batches by app and kind, and walks the list with one cursor. The\n"
        "right-to-explanation report pairs the action log with the provenance\n"
        "panel by action id, flags every action with no rationale, measures its\n"
        "own Flesch-Kincaid grade, and renders content only as a length and a\n"
        "hash. See\n"
        "[`../../docs/RULES_ENGINE.md`](../../docs/RULES_ENGINE.md),\n"
        "[`../../docs/PROFILE_GRAPH.md`](../../docs/PROFILE_GRAPH.md),\n"
        "[`../../docs/LAYOUT.md`](../../docs/LAYOUT.md),\n"
        "[`../../docs/GRID.md`](../../docs/GRID.md),\n"
        "[`../../docs/DIMMING.md`](../../docs/DIMMING.md),\n"
        "[`../../docs/CONFIRM.md`](../../docs/CONFIRM.md),\n"
        "[`../../docs/PROVENANCE.md`](../../docs/PROVENANCE.md),\n"
        "[`../../docs/STOP_MOTION.md`](../../docs/STOP_MOTION.md),\n"
        "[`../../docs/ONE_STEP.md`](../../docs/ONE_STEP.md),\n"
        "[`../../docs/INTERRUPTION_INBOX.md`](../../docs/INTERRUPTION_INBOX.md),\n"
        "[`../../docs/TYPOGRAPHY.md`](../../docs/TYPOGRAPHY.md),\n"
        "[`../../docs/RIGHT_TO_EXPLANATION.md`](../../docs/RIGHT_TO_EXPLANATION.md), and\n"
        "[`../../docs/RULES_ENGINE.md`](../../docs/RULES_ENGINE.md)."
    ),
    "curbcut-capture": (
        "The zeroizing frame ring buffer is implemented: a fixed-capacity\n"
        "ring with a byte budget, best-effort `mlock`, and a guard byte on\n"
        "release. Protected-content detection refuses a DRM surface, a\n"
        "secure desktop, an uncapturable window, or a black frame, and\n"
        "returns the accessibility-only fallback message. `GUARD_BYTE` is\n"
        "re-exported from `curbcut_core::secure`, so the ring and the other\n"
        "buffers share one guard byte. See\n"
        "[`../../docs/PROTECTED_CONTENT.md`](../../docs/PROTECTED_CONTENT.md) and\n"
        "[`../../docs/SECURE_BUFFERS.md`](../../docs/SECURE_BUFFERS.md)."
    ),
    "curbcut-cli": (
        "The headless CLI is implemented: status, redacted dump, benchmark,\n"
        "process-local forget-layouts, panic, vpat, and limitations commands,\n"
        "with stable exit codes. The accessibility mapping, the draft VPAT, and\n"
        "the limitations page are generated from one data module, so a published\n"
        "claim and the test that backs it cannot drift apart. See\n"
        "[`../../docs/CLI.md`](../../docs/CLI.md),\n"
        "[`../../docs/A11Y_MAPPING.md`](../../docs/A11Y_MAPPING.md), and\n"
        "[`../../docs/VPAT.md`](../../docs/VPAT.md)."
    ),
    "curbcut-vision": (
        "The vision pipeline is implemented: Detector, Ocr, and IconCaptioner\n"
        "traits, a deterministic motion-aware scheduler, and VisionPipeline.\n"
        "Motion classification reports a region as static, carousel, autoplay,\n"
        "or countdown once it has moved for long enough. Layout analysis divides\n"
        "a viewport into header, nav, main, aside, footer, modal, and floating\n"
        "regions and produces a stable reading order. Dirty-rect re-analysis\n"
        "reduces two frames to a grid of cell means, merges the changed cells\n"
        "into rectangles, and triggers re-analysis with a cooldown so a moving\n"
        "carousel causes one analysis rather than one per frame. `OcrText.text`\n"
        "and `IconCaption.caption` are zeroizing `SecureText`, so recognized\n"
        "content is overwritten when the result drops. The model guard pins each\n"
        "artifact by SHA-256, checks a release signature, and refuses to load a\n"
        "model that fails either check. See\n"
        "[`../../docs/VISION.md`](../../docs/VISION.md),\n"
        "[`../../docs/MOTION_DETECTION.md`](../../docs/MOTION_DETECTION.md),\n"
        "[`../../docs/LAYOUT_ANALYSIS.md`](../../docs/LAYOUT_ANALYSIS.md),\n"
        "[`../../docs/DIRTY_RECT.md`](../../docs/DIRTY_RECT.md), and\n"
        "[`../../docs/MODEL_GUARD.md`](../../docs/MODEL_GUARD.md)."
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
        "only. Icon semantics combines an icon's accessible label, its caption,\n"
        "and its neighbours into a plain what-this-does label of at most eight\n"
        "words, and marks a guess as uncertain. Extractive summarization ranks\n"
        "sentences by mean term frequency, skips near-duplicates, and emits key\n"
        "points in reading order with a byte range and chunk id per point, while\n"
        "ROUGE-1 and ROUGE-L recall are reported rather than gated. See\n"
        "[`../../docs/RULES.md`](../../docs/RULES.md),\n"
        "[`../../docs/READING_LEVEL.md`](../../docs/READING_LEVEL.md),\n"
        "[`../../docs/JARGON.md`](../../docs/JARGON.md),\n"
        "[`../../docs/TEXT_CHUNKING.md`](../../docs/TEXT_CHUNKING.md),\n"
        "[`../../docs/INTERRUPTIONS.md`](../../docs/INTERRUPTIONS.md),\n"
        "[`../../docs/ICON_SEMANTICS.md`](../../docs/ICON_SEMANTICS.md),\n"
        "[`../../docs/SUMMARIZE.md`](../../docs/SUMMARIZE.md),\n"
        "[`../../docs/NORMALIZE.md`](../../docs/NORMALIZE.md),\n"
        "[`../../docs/SENSITIVE_CONTENT.md`](../../docs/SENSITIVE_CONTENT.md), and\n"
        "[`../../docs/DESTRUCTIVE_ACTIONS.md`](../../docs/DESTRUCTIVE_ACTIONS.md), and\n"
        "[`../../docs/MEANING.md`](../../docs/MEANING.md)."
    ),
    "curbcut-store": (
        "The plan cache is implemented: a bounded LRU cache of overlay plans\n"
        "keyed by app, window, and content hash, and opt-in per-app memory for\n"
        "one remembered choice per app. The bounded LRU lives in `lru` and is\n"
        "shared with `understanding_cache`, which caches encoded understanding\n"
        "results keyed by content hash, model version, and profile, and stores\n"
        "every value as ciphertext from a caller-supplied `Cipher`. Per-user,\n"
        "per-app, per-window layout memory remembers a content-free choice,\n"
        "keeps it across an app update, resets per app, per user, or globally,\n"
        "and migrates a schema-v1 snapshot by filling the absent font scale.\n"
        "Learned simplifications record a content hash and an accept/reject\n"
        "verdict, move a simplification level by a bounded step only after a\n"
        "minimum number of observations and records, keep a bounded undo history,\n"
        "and report the acceptance rate. Sealed values are held in a zeroizing\n"
        "`SecureBuffer`, so eviction overwrites them rather than merely\n"
        "unlinking. See\n"
        "[`../../docs/PLAN_CACHE.md`](../../docs/PLAN_CACHE.md),\n"
        "[`../../docs/UNDERSTANDING_CACHE.md`](../../docs/UNDERSTANDING_CACHE.md),\n"
        "[`../../docs/LAYOUT_MEMORY.md`](../../docs/LAYOUT_MEMORY.md), and\n"
        "[`../../docs/LEARNED_SIMPLIFICATIONS.md`](../../docs/LEARNED_SIMPLIFICATIONS.md)."
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
        "scan, and a configurable dwell. Action consent is implemented:\n"
        "`ConsentGate` requires a per-session, per-action consent, states the\n"
        "consequence before the choice, scopes a receipt to one action or to the\n"
        "session, and logs action class and target role only. See\n"
        "[`../../docs/SHORTCUTS.md`](../../docs/SHORTCUTS.md),\n"
        "[`../../docs/HOVER.md`](../../docs/HOVER.md),\n"
        "[`../../docs/CLICKS.md`](../../docs/CLICKS.md),\n"
        "[`../../docs/FOCUS_ORDER.md`](../../docs/FOCUS_ORDER.md),\n"
        "[`../../docs/SWITCH_ACCESS.md`](../../docs/SWITCH_ACCESS.md), and\n"
        "[`../../docs/CONSENT.md`](../../docs/CONSENT.md)."
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
        "toggle microphone state machine. `Coexistence` is the arbiter with a\n"
        "screen reader: `ScreenReader` reports off/listening/speaking,\n"
        "`Ducking` is pause (the default) or a clamped duck level, and the\n"
        "arbiter never sounds over an announcement, holds rather than drops a\n"
        "request, and resumes it when the reader goes quiet. `simulate` runs a\n"
        "one-hour schedule and counts every overlap. `ReadingCursor` reads the\n"
        "current selection or reads from the pointer one sentence at a time,\n"
        "keeps the position across a stop and resume, and stores offsets only.\n"
        "See\n"
        "[`../../docs/SPEECH.md`](../../docs/SPEECH.md),\n"
        "[`../../docs/VOICE_COMMANDS.md`](../../docs/VOICE_COMMANDS.md),\n"
        "[`../../docs/COEXISTENCE.md`](../../docs/COEXISTENCE.md), and\n"
        "[`../../docs/READING_POSITION.md`](../../docs/READING_POSITION.md)."
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
    features = {"curbcut-app": ["tauri"], "curbcut-core": ["net"]}
    default_features = {"curbcut-core": []}
    for name, desc, deps in CRATES:
        d = crates / name
        (d / "src").mkdir(parents=True, exist_ok=True)
        (d / "Cargo.toml").write_text(
            crate_toml(
                name,
                desc,
                deps,
                features.get(name),
                default_features.get(name),
            ),
            encoding="utf-8",
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
