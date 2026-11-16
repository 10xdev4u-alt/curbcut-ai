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

Implemented in `crates/curbcut-a11y/src/merge.rs`. See
[`docs/MERGE.md`](MERGE.md).

For each a11y element:
- Match to a vision element by IoU + role compatibility.
- Merge: prefer a11y for name/role/state, vision for rect/text if a11y is silent.
- If they disagree, keep both with provenance and prefer the higher-confidence source; flag the conflict for the provenance panel.
- Unmatched vision elements become vision-only elements (canvas UIs, Electron).
- Unmatched a11y elements become "semantic-only" (invisible but focusable).

### 6.4 Stability tracking

Implemented in `crates/curbcut-a11y/src/stability.rs`. See
[`docs/STABILITY.md`](STABILITY.md).

- `StableId` comes from a Hungarian assignment on IoU plus label-hash agreement
  across frames; the cost matrix is padded to a square with a large finite cost
  so an unmatched observation becomes a new track.
- Scroll compensation estimates the delta from nearest-centre pairs, takes the
  component-wise median, clamps it to `max_scroll_jump`, and aligns each
  observation by it before matching.
- A track is dropped after `max_gap_frames` (default 30) frames without a match.
- `should_reanalyze(rect)` skips a region only when a live track covers it and no
  scroll motion was detected; `content_changed` compares label hashes.

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

### 7.4 Reading level scoring

Implemented in `crates/curbcut-understand/src/reading.rs`. See
[`docs/READING_LEVEL.md`](READING_LEVEL.md).

- `ReadingScore::compute` returns Flesch reading ease, Flesch-Kincaid grade,
  SMOG, and ARI, plus word, sentence, and syllable counts and `WordBands`.
- `classify_script` marks a text `Latin`, `NonLatin`, `Mixed`, or `Empty`.
  Only `Latin` is scored; every score is `None` otherwise.
- `smog` returns `None` below three sentences rather than reporting a grade it
  cannot support.
- `ScoreCache` is a bounded FIFO keyed by `content_hash`; the default limit is
  64 and the oldest entry is evicted when full.

### 7.5 Jargon dictionary

Implemented in `crates/curbcut-understand/src/jargon.rs`. See
[`docs/JARGON.md`](JARGON.md).

- `JargonEntry` carries a `Locale` and an `EntrySource`; neither has a default,
  so a change without a source or locale is not representable.
- `JargonDictionary::add` replaces an entry with the same locale and term and
  returns what it replaced, so a user entry overrides a built-in.
  `EntrySource::priority` orders user over community over built-in.
- Lookup is case-insensitive; an unknown term returns `None`.
- `expand_acronyms` walks a text, splits runs on non-alphanumerics, and reports
  `AcronymMatch` values with byte offsets. `is_acronym_token` accepts two to
  eight ASCII alphanumerics with at least one letter and no lowercase.
- An unknown acronym is flagged: `AcronymMatch::expansion` is `None` and
  `unknown_acronyms` lists it once. The dictionary never invents an expansion.

### 7.6 Number, date, and unit normalization

Implemented in `crates/curbcut-understand/src/normalize.rs`. See
[`docs/NORMALIZE.md`](NORMALIZE.md).

- `spell_integer` and `spell_number` produce words; `round_for_speech` rounds
  explicitly, so rounding is a caller decision and never changes meaning.
- `Money::parse` reads a symbol or an ISO code; whole amounts from one hundred
  to under one thousand that are exact hundreds speak as "twelve hundred
  dollars".
- `Date::parse` accepts `YYYY-MM-DD`, `YYYY/MM/DD`, and numeric forms where one
  field is greater than 12. `03/04/2026` is rejected as ambiguous.
  `Date::speak` is `16 November 2026`.
- `Measurement::speak` converts to the target unit and spells it;
  `MeasurementSystem::for_locale` is imperial only for `en-US`. Temperatures
  use the affine conversion.
- Every function is pure and stores nothing.

### 7.7 Sensitive-content guard

Implemented in `crates/curbcut-understand/src/sensitive.rs`. See
[`docs/SENSITIVE_CONTENT.md`](SENSITIVE_CONTENT.md).

- `SensitiveGuard::scan` classifies a text by `SensitiveDomain::{Financial,
  Medical, Legal}`. A strong signal triggers alone; two weak signals from the
  same domain trigger together. A single weak signal never does.
- `SensitiveSignal` is data: a marker, a domain, and a `SignalStrength`. The
  table is auditable through `SensitiveGuard::signals` and can be extended or
  cleared.
- Matching is case-insensitive. A single-word marker sits on word boundaries, so
  `bank` matches "river bank" but not "banking"; a multi-word marker is a
  phrase.
- `auto_simplify` checks both the original and the candidate and returns
  `Simplification::Original` whenever either is sensitive or the candidate is
  empty, so the original is always visible.
- `SensitiveReport` counts hits and names domains; it carries no text, and
  `message` is a fixed sentence naming domains.

### 7.8 Destructive-action preview

Implemented in `crates/curbcut-understand/src/actions.rs`. See
[`docs/DESTRUCTIVE_ACTIONS.md`](DESTRUCTIVE_ACTIONS.md).

- `ActionCatalog` is versioned (`CATALOG_VERSION`) and auditable through
  `actions()`. The shipped set covers delete, permanent delete, overwrite,
  send, purchase, cancel subscription, revoke, format, publish, transfer, and
  end session.
- `DestructiveAction` carries a `phrase`, a `consequence`, a `reversible` flag,
  and a `triggers` list; `summary()` builds the plain preview sentence.
- `detect` is case-insensitive, matches single-word triggers on word boundaries
  and multi-word triggers as phrases, and resolves overlaps by longest trigger
  so "permanently delete" beats "delete".
- `confirm` returns `Confirmation::Required(preview)` for a detected action and
  `NotRequired` otherwise. `can_proceed` is true only for `NotRequired`; the
  catalog has no perform method that skips the preview.

### 7.9 Meaning preservation

Implemented in `crates/curbcut-understand/src/meaning.rs`. See
[`docs/MEANING.md`](MEANING.md).

- `MeaningChecker::check` extracts named entities, numbers, negations, and
  comparatives from the original and the candidate, then counts the original
  tokens with no counterpart in the candidate as a multiset.
- Any missing token is a rejection: `Preserved` or `Rejected(MeaningReport)`.
  `MeaningReport` holds counts in the four categories; `recall` is
  `1 - missing / original`.
- `rewrite` returns the candidate only when it is non-empty and preserved, and
  otherwise returns the original.
- `RejectionLog` accumulates rejection reasons as counts; `describe` is a fixed
  sentence built from reason labels and counts, so a rejection is logged without
  quoting the screen.

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

### 8.7 Stop-motion mode

Implemented in `crates/curbcut-adapt/src/stop_motion.rs`. See
[`docs/STOP_MOTION.md`](STOP_MOTION.md).

- `StopMotionPlanner::plan` keeps the motion regions whose `MotionKind` is
  enabled by `StopMotionConfig` (carousels, autoplay, countdowns), sorts them by
  `StableId`, and gives each a `FrozenRegion` with a content-free snapshot hash.
- The plan only covers a region; it never pauses, seeks, or otherwise controls
  the underlying media. There is no such method.
- `StopMotionPlan::restore` lifts one snapshot, `restore_all` lifts the rest,
  and `should_cover(rect)` reports whether a covered region intersects a rect.
- `StopMotionSession` pairs the config with the plan and an enabled flag;
  `should_cover` is false while disabled, and disabling lifts every snapshot.

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

### 9.1 Focus surfaces

Implemented in `crates/curbcut-render/src/focus.rs`. See
[`docs/FOCUS.md`](FOCUS.md).

- `ReadingRuler` highlights one to three text lines (`line_count` clamped to
  `1..=3`) and spans the screen width; `move_to`/`rect` clamp the band to the
  screen.
- `FocusLens` magnifies a region. `update` leads the pointer in the direction of
  travel by `lead_px`, moves the centre halfway to the target, and rejects a move
  inside `dead_zone_px` so the lens does not jitter.
- `source_rect` is the magnified region: a square of side
  `screen.width / magnification`, clamped to the screen.
- Both surfaces return `None` under `MotionMode::Reduced`.

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

### 10.4 Click-through routing

Implemented in `crates/curbcut-input/src/click.rs`. See
[`docs/CLICKS.md`](CLICKS.md).

- `ClickRouter` is the single decision point for a click: `route` returns
  `Consume` or `Handoff`, never both, so a click is never delivered twice.
- Handoff is never accidental: only `ClickTarget::App` can hand off, and a click
  on the overlay is always consumed.
- A handoff owes focus. `focus_due` reports the outstanding `HandoffToken` and
  `return_focus` clears it; focus stays in the app until the overlay takes it
  back.
- A second handoff inside `double_click_ms` is suppressed and counted, so the
  app never receives a double-click. A backwards timestamp counts as inside the
  window.
- The router stores a timestamp, a token id, and counters. The target is a
  two-variant enum, not a coordinate, and no click position is stored.

### 10.5 Provenance panel

Implemented in `crates/curbcut-adapt/src/provenance.rs`. See
[`docs/PROVENANCE.md`](PROVENANCE.md).

- `ProvenancePanel` records one entry per change. `record` requires a `RuleId`,
  so a change without a rule is not representable.
- `ProvenanceEntry` links the `ActionId`, the `RuleId`, the `SettingKey`, the
  `OriginalValue`, and the time. `get`/`original` are the one-click lookup.
- `OriginalValue::Setting` keeps a user-chosen setting value whole.
  `OriginalValue::Content` keeps only a length and a non-cryptographic identity
  hash, so content-derived originals are never retained as text.
- `OriginalValue::describe` is the log-safe summary; it never contains the value
  or the text it stands for.
- `for_action` and `for_rule` filter without copying; `entries` is oldest
  first.

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

### 11.3 Telemetry

Implemented in `crates/curbcut-telemetry`. See [`docs/TELEMETRY.md`](TELEMETRY.md).

- `MetricKey::ALL` is a 16-entry allowlist; `MetricKey::from_name` returns `None`
  for any other name, so a metric cannot be filed under a key that would carry
  content, a label, or a session identifier.
- `Snapshot` is a `BTreeMap<MetricKey, u64>`. `serialize` writes a
  `schema_version` marker and one `key=value` line per key in declaration order;
  `parse` reads it back and rejects an unknown key, a malformed line, or a
  different version.
- `TelemetryExporter::export` is `None` until `opt_in`; `opt_out` disables and
  clears the counts; `set_enabled` keeps them.
- No timestamp, no session id, no network call. `Debug` prints key names and
  numbers only.

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

---

## 16. Speech and voice

Implemented in `crates/curbcut-tts`. See [`docs/SPEECH.md`](SPEECH.md).

- `TtsEngine` is the adapter trait. `ScriptEngine` is the deterministic local
  fallback: always available, `start_latency_ms` 5, no network, and it returns a
  `SpeechAudio` handle (sample rate, duration, content hash) rather than
  samples.
- `FallbackChain::speak` walks the engines in order and returns the first
  success. A candidate is skipped when it is unavailable, its start latency
  exceeds `LatencyBudget` (default 50 ms), or `uses_network` is true.
- `VoiceCatalog::can_download` is false until the user opts in or the voice is
  on disk; `DownloadPolicy::Forbidden` is the default.
- `AudioFocus` is a state machine (`Idle`, `Requested`, `Granted`, `Abandoned`)
  with one holder at a time; a grant without a request is ignored.
- `Utterance` implements `Debug` with the text redacted and `describe` reports a
  length and a hash. No speech text is logged.
- `command::CommandGrammar` matches a normalized utterance against a versioned
  phrase catalog, keeping the longest phrase that matches at a word boundary;
  the remainder is the argument. An unmatched utterance is `VoiceCommand::Unknown`
  and is answered with a line that points at the help command.
- `command::CommandGrammar::help` renders the "what can I say?" panel.
- `command::PushToTalk` is a hold-or-toggle microphone state machine; `press`,
  `release`, and `toggle` change the state, and `transitions` counts changes.
- Platform adapters for Piper, AVSpeechSynthesizer, SAPI, and espeak-ng are not
  implemented; the abstraction and the local fallback are.
