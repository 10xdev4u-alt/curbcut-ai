# Architecture

> This is the target architecture. Phase 0–1 builds the skeleton; later phases fill it. Where a choice is provisional, it says so.

---

## 1. Guiding decisions

| Decision | Choice | Rationale |
| --- | --- | --- |
| Shell | Tauri 2 (Rust + system webview) | Small binaries, memory safety, native plugin system, cross-platform. |
| Core language | Rust | Capture/a11y/input need low-level APIs; Rust has the best bindings and no GC pauses. |
| Frontend | Svelte 5 + TypeScript | Small runtime, fine-grained reactivity, good for a transient overlay. |
| Inference | ONNX Runtime via `ort` | Cross-platform, CoreML/DirectML/QNN/XNNPACK, quantized models. |
| Training | Python + PyTorch | Model iteration; exports to ONNX. Not shipped to users. |
| Storage | SQLite (via `sqlx` or `rusqlite`), encrypted with SQLCipher/Stronghold | Local, robust, queryable, encryptable. |
| Messaging | In-process event bus (tokio broadcast + typed channels); IPC via Tauri commands/events | Typed, testable, no serialization surprises. |
| Config | Versioned TOML + SQLite overrides | Human-readable defaults, user overrides in DB. |
| Updates | Tauri updater, signed, reproducible | Supply-chain safety. |
| License | AGPL-3.0 core + commercial exception | See RESEARCH. |

---

## 2. Process model

```
┌──────────────────────────────────────────────────────────────────────┐
│                         CurbCut AI (one user session)                  │
│                                                                        │
│  ┌────────────┐   frames   ┌──────────────┐   elements   ┌──────────┐ │
│  │  capture   │ ─────────► │   vision     │ ───────────► │  merge   │ │
│  │  (OS)      │            │ (ONNX RT)    │              │  engine  │ │
│  └────────────┘            └──────────────┘              └────┬─────┘ │
│       ▲                                                       │       │
│       │                                                       ▼       │
│  ┌────────────┐   tree    ┌──────────────┐   text     ┌──────────┐  │
│  │  a11y      │ ────────► │  understand  │ ◄───────── │  graph   │  │
│  │  (OS)      │           │ (rules+LLM)  │            │  store   │  │
│  └────────────┘           └──────┬───────┘            └────┬─────┘  │
│                                  │                         │        │
│                                  ▼                         ▼        │
│                           ┌──────────────┐          ┌──────────┐   │
│                           │   adapt      │ ───────► │  plan    │   │
│                           │ (profiles)   │          │  cache   │   │
│                           └──────────────┘          └────┬─────┘   │
│                                                         │         │
│                              ┌────────────┐             ▼         │
│                              │  render    │ ◄──────── ┌──────────┐  │
│                              │ (Tauri UI) │           │ overlay  │  │
│                              └────────────┘           └──────────┘  │
│                                     │                               │
│                              ┌──────┴──────┐                        │
│                              │  tts / input│                        │
│                              └─────────────┘                        │
└──────────────────────────────────────────────────────────────────────┘
```

One process, several tokio tasks, one event bus. No separate daemon (simpler, no privilege separation headaches), but with a strict internal capability model: only the capture task can read frames, only the input task can inject events, and only the render task can draw.

**Why not a sidecar Python process for models?** Because ONNX Runtime runs natively in Rust, and a second process doubles IPC and memory. Python is for training only. Exception: if we adopt a model that has no Rust path (e.g., a specific LLM runtime), a sandboxed sidecar with a typed stdin/stdout protocol is allowed. Document it in ADR-014.

---

## 3. Crate layout

```
crates/
  curbcut-core/         Event bus, config, errors, capability tokens, orchestration
  curbcut-capture/      Screen capture adapters + frame ring buffer
  curbcut-a11y/         Accessibility tree adapters + element model
  curbcut-vision/       ONNX sessions, detection, OCR, layout, saliency, motion
  curbcut-understand/   Classification, simplification, reading level, icon labels
  curbcut-adapt/        Profiles, rules engine, layout solver
  curbcut-render/       Overlay renderer (Tauri), interaction, animations
  curbcut-tts/          Piper/espeak, audio focus, voice commands
  curbcut-input/        Global shortcuts, hover/click routing, switch access
  curbcut-store/        SQLite schema, migrations, repositories, encryption, plan cache
  curbcut-telemetry/    Opt-in, anonymized, local-only metrics
  curbcut-cli/          Headless control, benchmark, debug dump
  curbcut-app/          Tauri app shell + Svelte frontend
  curbcut-macros/       Typed event/provenance macros
```

Rules:

- Each crate < 5k LOC by default. Split further when it grows.
- No crate depends on the frontend.
- Adapters are trait objects behind `Platform` enums; tests use a fake platform.
- Every crate has a `README.md` and a `docs/adr/` entry for non-obvious choices.

---

## 4. Core types

```rust
// crates/curbcut-core/src/types.rs (sketch)

pub struct Frame {
    pub id: FrameId,
    pub display_id: DisplayId,
    pub window_id: Option<WindowId>,
    pub captured_at: Instant,
    pub size: Size,
    pub buffer: FrameBuffer,          // zeroized on drop
    pub provenance: Provenance,       // which API produced it
}

pub struct Element {
    pub id: ElementId,
    pub rect: Rect,                   // screen coordinates
    pub role: Role,                   // button | link | text_field | ...
    pub label: Option<String>,        // from a11y tree or vision or both
    pub text: Option<String>,         // OCR text if any
    pub state: ElementState,          // enabled, focused, selected, ...
    pub confidence: f32,
    pub source: SourceSet,            // a11y | vision | merged
    pub app: AppId,
    pub stable_id: StableId,          // tracks across frames
}

pub struct ScreenGraph {
    pub frame_id: FrameId,
    pub elements: Vec<Element>,
    pub regions: Vec<Region>,
    pub order: Vec<ElementId>,        // reading order
    pub media: Vec<Media>,
    pub interruptions: Vec<Interruption>,
    pub reading_level: ReadingLevel,
}

pub struct OverlayPlan {
    pub profile: ProfileId,
    pub actions: Vec<Action>,          // dim, grid, reflow, chunk, speak, ...
    pub layout: Layout,
    pub rationale: Vec<Rationale>,     // "why" for the provenance panel
}

pub enum Action {
    Dim { region: Rect, except: Vec<Rect> },
    IconGrid { source: Rect, tiles: Vec<Tile> },
    Reflow { text: String, style: TextStyle },
    Chunk { text: String, parts: Vec<String> },
    Speak { target: ElementId, rate: f32 },
    Freeze { target: Rect },
    Inbox { items: Vec<Interruption> },
    StepMode { steps: Vec<Step> },
    Highlight { target: Rect, style: HighlightStyle },
}
```

All types are `Serialize` for tests and CLI dumps, but **never for persistence of content**.

---

## 5. Event model

Typed events flow through a broadcast bus with per-consumer backpressure. No event carries raw pixels except `FrameCaptured`, which stays in-process.

| Event | Producer | Consumers |
| --- | --- | --- |
| `FrameCaptured` | capture | vision, render (throttled) |
| `A11yTreeChanged` | a11y | merge |
| `ElementsDetected` | vision | merge, understand |
| `TextRecognized` | vision | understand |
| `GraphMerged` | merge | understand, adapt, store |
| `ContentAnalyzed` | understand | adapt |
| `ProfileChanged` | render/input | adapt, render |
| `PlanBuilt` | adapt | render, store |
| `OverlayAction` | render | input, adapt |
| `SpeechRequested` | render/input | tts |
| `SpeechFinished` | tts | render |
| `AppChanged` | capture | adapt, store |
| `PermissionChanged` | core | all |
| `PanicRequested` | input | all |

Backpressure policy: slow consumers drop non-critical events (vision frames) and keep critical ones (profile changes, panic). Every event has a `TraceId` for debugging without content.

---

## 6. Perception pipeline

### 6.1 Capture

- **Mode A — active window**: capture the focused window at up to 2 fps for analysis, 15–30 fps for overlay tracking.
- **Mode B — full display**: for multi-window and floating-panel layouts.
- **Mode C — region of interest**: after the first analysis, re-capture only changed regions using a dirty-rect heuristic.
- Frame ring buffer: 2 seconds, zeroized on release, `mlock` where the OS allows.
- Motion detection: frame differencing on a downscaled grayscale buffer; triggers re-analysis only on significant change.

### 6.2 Vision

1. **Detector** (YOLOv9-E int8) → candidate regions + class.
2. **OCR** (PP-OCRv6) → text boxes, text, confidence, reading order.
3. **Layout** (heuristic + optional small model) → regions, reading order, grouping.
4. **Icon caption** (Florence-2-base or Qwen2.5-VL-3B int4) → only for icons without a11y labels, cached by icon crop hash.
5. **Media/motion** → detect video/carousel/countdown from pixels + a11y roles.

### 6.3 A11y merge

For each a11y element:
- Match to a vision element by IoU + role compatibility.
- Merge: prefer a11y for name/role/state, vision for rect/text if a11y is silent.
- If they disagree, keep both with provenance and prefer the higher-confidence source; flag the conflict for the provenance panel.
- Unmatched vision elements become vision-only elements (canvas UIs, Electron).
- Unmatched a11y elements become "semantic-only" (invisible but focusable).

### 6.4 Stability tracking

- `StableId` from a Hungarian match on IoU + label similarity across frames.
- Re-analyze a region only when its content hash changes or motion is detected.
- Scroll compensation: shift element rects by the detected scroll delta.

---

## 7. Understanding pipeline

### 7.1 Rules layer (deterministic, always available)

- Role refinement from a11y role + visual cues.
- Reading level: Flesch-Kincaid, SMOG, ARI, syllable count, word-frequency bands.
- Jargon dictionary (open, versioned, per-locale) + acronym expansion.
- Number/date/unit normalization ("$1,200" → "twelve hundred dollars").
- Chunking: sentence split, clause split, list detection, heading detection.
- Distraction heuristics: motion, autoplay, countdown, animation, toast frequency.

### 7.2 Model layer (local, cached)

- **Simplify**: int4 instruct model, prompt with reading-level target and domain constraints, output JSON `{simplified, changed, warnings}`.
- **Summarize**: extractive + abstractive, chunked, with a "key points" list.
- **Icon label**: caption model + context (app, neighbors, role).
- **Meaning preservation**: compare named entities, numbers, and negation between original and simplified; if mismatch, reject and fall back to the original with a warning.

### 7.3 Cache

Keyed by `sha256(normalized_content + model_version + profile)`. LRU with a size cap. Cache hits must be deterministic; a cache miss must not change behavior, only latency.

---

## 8. Adaptation engine

### 8.1 Profile graph

```toml
[profile.focus]
dim = { enabled = true, opacity = 0.75, except = ["active_region"] }
stop_motion = true
interruptions = "inbox"
icon_grid = { min_tile = 72, columns = 3 }
short_paths = true
reading_level = "keep"
tts = { rate = 1.1 }

[profile.read]
reflow = { font = "Atkinson Hyperlegible", size = 20, line_height = 1.6, align = "left", line_length = 38 }
ruler = true
read_aloud = { highlight = "word", rate = 0.9 }
reading_level = "grade-8"

[profile.simple]
icon_grid = { min_tile = 88, columns = 2, symbols = true }
one_step = true
memory_aids = true
reading_level = "grade-6"
confirm_actions = true
```

The graph is implemented in `curbcut-adapt`. `SettingKey` is the closed registry
of the forty settings a graph can express, each with a type and range, and
`ProfileGraph` applies changes atomically and notifies live-preview sinks after
a commit. See [`docs/PROFILE_GRAPH.md`](PROFILE_GRAPH.md).

### 8.2 Rules engine

Forward-chaining over the `ScreenGraph` with priority and conflict resolution:

```
rule "carousel_is_distraction"
  when media.kind == carousel and profile.stop_motion
  then Freeze(target) and Rationale("carousel paused")

rule "dense_menu_to_grid"
  when region.role == menu and element_count > 8 and profile.icon_grid
  then IconGrid(...)

rule "long_text_chunk"
  when text.reading_level > profile.reading_level
  then Chunk(...) and optionally Simplify(...)

rule "destructive_confirm"
  when element.role == button and label in DESTRUCTIVE
  then Confirm(...)
```

Rules are data, versioned, and testable. Users can disable individual rules.

### 8.3 Layout solver

Implemented in `crates/curbcut-adapt/src/layout.rs`. See
[`docs/LAYOUT.md`](LAYOUT.md).

- Input: a viewport `Rect`, a list of `LayoutElement` (id, priority, reading
  order, minimum size, preferred size), and a gap.
- Output: `LayoutSolution` with `placed` boxes and `dropped` element ids.
- `place_tiles` is a priority-based shelf packer, not a general CSP: big
  targets first, reading order second, no overlap and no clipping third. A box
  that does not fit the remaining row wraps; a box that does not fit the
  viewport height shrinks toward its minimum, then is dropped.
- `reflow` breaks measured words into lines. A word is never split, so a word
  wider than the measure is reported as an overflowing line rather than clipped.
- Pure and deterministic: no I/O, no clock, no panics. 10,000 fuzz cases assert
  the invariants; p95 solve time is printed by the test.
- Animate only when `prefers-reduced-motion` is false.

### 8.4 Icon grid

Implemented in `crates/curbcut-adapt/src/grid.rs`. See
[`docs/GRID.md`](GRID.md).

- Input: a viewport `Rect`, a list of `GridTile` (id, label, reading order),
  and the profile's `IconGrid` settings.
- Output: `GridLayout` with the honoured column count, the square tile size,
  `placed` rectangles in row-major reading order, and `dropped` tile ids.
- A tile is never smaller than `icon_grid.min_tile` (72 pt by default). The
  column count is reduced until that holds; a viewport that cannot fit one
  minimum tile drops everything rather than clipping.
- `validate_label` enforces a plain, non-empty label of at most eight words;
  `search_tiles` filters by an AND of case-insensitive tokens in reading order.
- Pure and deterministic, and total on extreme viewports.

### 8.5 Dimming

Implemented in `crates/curbcut-adapt/src/dim.rs`. See
[`docs/DIMMING.md`](DIMMING.md).

- Input: a screen `Rect`, the focused control, the except-regions, the profile
  opacity, and a `DimMotion`.
- Output: `DimPlan` with the scrim `rects`, the validated `opacity`, the
  `motion`, and the clipped `protected` rectangles.
- The focused control and every except-region are never dimmed. The scrim
  covers the screen minus the protected union exactly, with no overlap.
- The complement is computed by coordinate compression and a 2D difference
  array; `DimPlan::covers_exactly` checks the area identity and the tests
  sample the plane.
- `DimMotion::Static` is the reduced-motion path. The screen is clamped to the
  `i32` range, so extreme geometry is total.

### 8.6 Plan cache

Implemented in `crates/curbcut-store/src/plan_cache.rs`. See
[`docs/PLAN_CACHE.md`](PLAN_CACHE.md).

- `PlanCache<V>` is a bounded LRU cache of overlay plans. The key is a
  `PlanKey`: an `AppId`, a `WindowId`, and a `ContentHash`. The capacity is the
  caller's choice; `put` evicts the least recently used entry when full, and
  `get` marks a hit most-recently-used.
- A scroll reuses the plan when the cached value is content-relative, because a
  scroll does not change the `ContentHash`. `OverlayPlan` rects are
  screen-space, so a consumer caching one must translate the rects by the
  scroll delta (§6.4) or include the viewport in the key. A content change is a
  new key and therefore a miss. `invalidate_app`, `invalidate_window`, and
  `invalidate_content` drop entries when the context itself is gone.
- `ContentHash` is a `u64` wrapper. The producer hashes normalized content; the
  cache never sees the bytes.
- `AppMemory<V>` remembers one user choice per app and is opt-in:
  `remember` stores nothing for an app that was not enabled with `enable_app`,
  and `disable_app` forgets the choice as well as the opt-in.
- Pure data structures: no I/O, no clock, and no reachable panics. The caller
  owns the privacy boundary for `V`; both containers derive `Debug`.

---

## 9. Rendering

Two surfaces:

1. **In-place overlay** — absolutely-positioned panels over the real app. Used for dimming, ruler, highlight, icon grid, toasts inbox.
2. **Simplified view** — a separate borderless window with a clean layout. Used for one-step mode, reading, and complex reflows.

Rendering rules:

- Never cover the focused control without a visible way to dismiss.
- Click-through by default; our controls are hit-testable and stop propagation.
- All text is real text (selectable, searchable, a11y-exposed), never baked into images.
- Animations capped at 150 ms and disabled under reduced motion.
- Colors and fonts come from the profile; the overlay never inherits the app's colors (they may be the problem).

---

## 10. Input and action model

- **Observe** (default): capture, analyze, suggest. Never acts.
- **Assist** (per-session opt-in): can speak, scroll, highlight, move focus.
- **Act** (per-action opt-in): can click/type, only after an explicit confirmation, only for the specific action, only in the current session.
- Every action is logged (action type, target role, timestamp — never content) and undoable where the app allows.
- Panic key: `Ctrl+Shift+Esc` (configurable) → hide overlay, stop capture, mute audio, restore app.

### 10.1 Global shortcut catalog

Implemented in `crates/curbcut-input/src/shortcuts.rs`. See
[`docs/SHORTCUTS.md`](SHORTCUTS.md).

- `ShortcutAction` is the closed set of twelve global actions; each has a
  default chord and a one-line purpose.
- `ShortcutCatalog` holds the bindings. `action_for` maps a pressed chord to an
  action, comparing chords as sets (order- and case-insensitive);
  `rebind` refuses a chord another action already uses; `conflicts` lists any
  pair that shares one.
- The default catalog has no conflicts. `curbcut_core::config::is_known_key`
  is public so the catalog and the configuration validator agree on key names.

### 10.2 Confirmation and undo

Implemented in `crates/curbcut-adapt/src/confirm.rs`. See
[`docs/CONFIRM.md`](CONFIRM.md).

- `Confirmation` states the plain-language consequence of a destructive action
  and whether it is undoable; `offers_undo` is the single source of truth.
- `ActionLog` is append-only. `record` stores the `Outcome` (`Applied`,
  `Dismissed`, or `Undone`) with the `RuleId` that authorised the action.
- `undo_last` undoes the most recent applied, undoable, not-yet-undone action
  and returns its id. A dismissed or permanent action is never undone, and an
  action is never undone twice.
- The log holds ids, rule references, and outcomes only — never content.

### 10.3 Pointer tracking

Implemented in `crates/curbcut-input/src/hover.rs`. See
[`docs/HOVER.md`](HOVER.md).

- `HoverTracker` consumes `PointerSample` values and classifies movement, so
  hover-to-read and the lens wait for a settled pointer.
- A hover is dwell, not presence: the pointer must stay within `move_threshold`
  for `settle_ms` before the state is `Settled`. A move is per-axis, and fewer
  than two samples is `Unknown`.
- `is_over` hit-tests the last position against a `Rect` with the half-open
  `Rect::contains`.
- The tracker stores the last sample and counts only. `sample_count`,
  `move_count`, and `path_length_px` are the telemetry; the position is not
  logged.
- macOS and Windows support global tracking; Wayland does not, and the
  limitation is documented rather than hidden.

---

## 11. Storage and privacy

### 11.1 Data inventory

| Data | Where | Encryption | Retention |
| --- | --- | --- | --- |
| Frames | RAM ring buffer | mlock where possible | 2 seconds |
| Element graph | RAM | n/a | session |
| Text/OCR | RAM + model cache | encrypted DB | until cache eviction |
| Profiles/settings | SQLite | SQLCipher | until deleted |
| App layouts | SQLite | SQLCipher | until deleted |
| Learned simplifications | SQLite | SQLCipher | until deleted |
| Logs | disk | redacted | 7 days, no content |
| Telemetry | local only | n/a | opt-in, 90 days |

### 11.2 Rules

- No raw pixels in logs, crash reports, or analytics.
- No raw text in logs. Log hashes and counts.
- Cache encryption at rest with a key in the OS keychain (macOS Keychain, Windows DPAPI, Linux libsecret).
- Memory-zeroing for frame buffers and text caches on drop.
- Network access is compiled in only for updates and optional sync; a runtime kill-switch disables it.

---

## 12. Model serving

- One `ort::Session` per model, loaded lazily, kept warm.
- Int8 for detection and OCR, int4 for the LLM, fp16 for captioning if accuracy demands.
- Execution provider order: CoreML (macOS) / DirectML (Windows) / QNN (ARM) / CUDA/TensorRT / OpenVINO / XNNPACK.
- Fallback chain: quantized → fp16 → fp32 → heuristic. Never fail silently.
- Model files signed and pinned by hash; update through the app updater.
- A `models/manifest.json` records version, hash, license, source, and benchmark results.

---

## 13. Testing strategy

- **Unit**: pure functions (rules, layout, reading level, merge). High coverage.
- **Property-based**: merge and layout invariants (no overlap, reading order preserved, idempotent).
- **Golden**: snapshot tests for overlay plans against a fixture screen dataset.
- **Differential**: compare our merge against the raw a11y tree and against a screen reader's reading order.
- **Integration**: headless platform fakes that emit synthetic frames/trees.
- **E2E**: Playwright for our UI; a scripted app (Electron test harness) for overlay behavior.
- **Perf**: benchmark suite on three reference machines (M-series Mac, Windows iGPU, Linux iGPU) with budgets.
- **A11y**: automated (axe, platform a11y audit) + manual with screen readers and switch users.
- **Chaos**: kill permissions, revoke mid-session, corrupt cache, huge screens, 4K/5K, mixed DPI, RDP.

---

## 14. Build and release

- Rust workspace, `cargo workspaces`, locked deps, `cargo-deny` for licenses, `cargo-audit` for CVEs.
- CI on macOS (arm64 + x64), Windows (x64), Linux (x64 + arm64) with real runners.
- Frontend: `pnpm`, `vitest`, `playwright`, `svelte-check`.
- Reproducible builds: pinned toolchain, vendored deps, signed artifacts, SBOM.
- Release channels: `nightly`, `beta`, `stable`. Auto-update with rollback.

---

## 15. Architecture decision records

ADRs live in `docs/adr/`. Initial set:

- `ADR-001` Use Tauri 2, not Electron.
- `ADR-002` Rust core, Python training only.
- `ADR-003` ONNX Runtime, not Torch/TensorFlow, at runtime.
- `ADR-004` Local-first, no cloud for perception.
- `ADR-005` Vision + a11y merge, vision as fallback.
- `ADR-006` Read-only by default; action consent per session.
- `ADR-007` AGPL-3.0 core + commercial exception.
- `ADR-008` SQLite + SQLCipher, no server.
- `ADR-009` Svelte 5 for the overlay UI.
- `ADR-010` No telemetry on by default.
- `ADR-011` Panic key is a hard requirement.
- `ADR-012` Model cache keyed by content hash + version.
- `ADR-013` Never auto-simplify financial/medical/legal content.
- `ADR-014` Sidecar processes allowed only for models with no Rust path, behind a typed protocol.
