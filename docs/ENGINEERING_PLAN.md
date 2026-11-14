# Engineering Plan

> Bit-by-bit execution plan. Each phase has a goal, a set of work packages, an exit criterion, and a rollback plan. Phases are not sprints; they are gated stages. A phase ends when its exit criterion is met, not when a date passes.

---

## How to read this

- **WP-xx** = work package. Each WP is small enough for one engineer in 1–5 days.
- **Exit criterion** is measurable. "Works" is not a criterion; "detects ≥ 0.85 mAP on the fixture set" is.
- **Rollback** is planned before the work starts.
- Dependencies are explicit. No phase starts with an unmet dependency.

---

## Phase 0 — Foundations (weeks 1–3)

**Goal:** agree on what we are building, for whom, and what we will never do. Set up the repo, governance, and the measurement harness.

### Work packages

| WP | Title | Deliverable | Owner |
| --- | --- | --- | --- |
| WP-0.1 | Repository + CI skeleton | `curbcut-ai` repo, Rust workspace, Tauri app stub, GitHub Actions on 3 OSes | Platform |
| WP-0.2 | Governance | `AGENTS.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `LICENSE` (AGPL-3.0), CLA | Founder |
| WP-0.3 | Advisory panel | 6–10 paid members with cognitive/learning disabilities, contract, quarterly review | Founder |
| WP-0.4 | Ethics charter | `docs/ETHICS.md`: what we will never do, what requires human review | Founder + panel |
| WP-0.5 | Fixture dataset v0 | 500 screen captures across 20 apps, 3 OSes, labeled elements, with consent and licensing | Vision |
| WP-0.6 | Benchmark harness | `curbcut-bench`: detection mAP, OCR CER, latency, memory, battery | Platform |
| WP-0.7 | Threat model v1 | `docs/THREAT_MODEL.md`, data inventory, DPIA template | Security |
| WP-0.8 | Design system v0 | Color tokens, type scale, spacing, motion rules, dark/light, reduced-motion | Design |
| WP-0.9 | ADR process | `docs/adr/`, ADR-001..014 | Architect |
| WP-0.10 | Success metrics | Dashboard spec + event schema (local only) | Data |

### Exit criterion

- CI green on macOS arm64/x64, Windows x64, Linux x64.
- Fixture dataset licensed, documented, and loadable by the bench.
- Bench produces a baseline report (even if all zeros).
- Advisory panel contracted and has reviewed `ETHICS.md`.
- Threat model reviewed by an external security reviewer.

### Rollback

Phase 0 is mostly documents and scaffolding. If the panel or the threat model surfaces a fundamental conflict, we stop and re-scope before writing product code.

---

## Phase 1 — Platform core (weeks 4–7)

**Goal:** a running app that can capture a screen, show a transparent overlay, respond to a global shortcut, and store settings — with no perception yet.

### Work packages

| WP | Title | Deliverable | Depends on |
| --- | --- | --- | --- |
| WP-1.1 | Event bus + typed errors | `curbcut-core`, `TraceId`, backpressure policy | 0.1 |
| WP-1.2 | Config + profile graph | TOML schema, defaults, validation, hot reload | 1.1 |
| WP-1.3 | Capture adapters | macOS ScreenCaptureKit, Windows Graphics Capture, Linux PipeWire; fake adapter | 1.1 |
| WP-1.4 | Frame ring buffer | Zeroized buffer, mlock, dirty-rect heuristic, 2 fps default | 1.3 |
| WP-1.5 | A11y adapters | macOS AX, Windows UIA, Linux AT-SPI; element model; fake adapter | 1.1 |
| WP-1.6 | Merge engine v0 | IoU + role matching, provenance, conflict flags | 1.3, 1.5 |
| WP-1.7 | Overlay window | Transparent, borderless, always-on-top, click-through, multi-display, DPI aware | 1.1 |
| WP-1.8 | Global shortcuts + panic key | Register/unregister, conflict detection, `Ctrl+Shift+Esc` panic | 1.7 |
| WP-1.9 | Settings UI | Profile picker, per-feature toggles, permission rationale | 1.2, 1.7 |
| WP-1.10 | Local store | SQLite schema v1, migrations, encryption, repositories | 1.2 |
| WP-1.11 | CLI + debug dump | `curbcut-cli status/dump/bench`, no content in dumps by default | 1.1 |
| WP-1.12 | Onboarding v0 | Permission walkthrough, profile picker, 2-minute tutorial | 1.9 |

### Exit criterion

- On all three OSes: capture the active window at 2 fps, show a transparent overlay, toggle it with a shortcut, panic-hide it, and persist a profile.
- Idle resource budget: < 150 MB RAM, < 5% CPU, < 1%/min battery on the reference machines.
- 100% unit coverage of the merge engine and config validation.
- A new contributor can build and run from a clean checkout in < 15 minutes (documented).

### Rollback

If PipeWire capture proves unusable on a target distro, we ship Linux as "beta, X11 only" and gate the Linux release. If Tauri's overlay cannot be click-through on Windows, we fall back to a topmost window with explicit focus handoff.

---

## Phase 2 — Perception (weeks 8–14)

**Goal:** from a frame, produce a `ScreenGraph` that is good enough to drive a first adaptation.

### Work packages

| WP | Title | Deliverable | Depends on |
| --- | --- | --- | --- |
| WP-2.1 | Model manifest + downloader | Pinned hashes, license metadata, signed updates | 0.5 |
| WP-2.2 | Detector integration | YOLOv9-E int8 via `ort`, NMS, class map | 1.4, 2.1 |
| WP-2.3 | OCR integration | PP-OCRv6, reading order, confidence, per-language models | 1.4, 2.1 |
| WP-2.4 | Layout analysis | Regions, reading order, grouping heuristics | 2.2, 2.3 |
| WP-2.5 | Merge engine v1 | Full provenance, conflict resolution, stability tracking | 1.6, 2.2 |
| WP-2.6 | Motion/media detection | Frame differencing, carousel/countdown/autoplay heuristics | 2.4 |
| WP-2.7 | Icon captioning | Florence-2/Qwen2.5-VL, cached by crop hash | 2.2 |
| WP-2.8 | Reading level | FK, SMOG, ARI, word frequency, grade mapping | 2.3 |
| WP-2.9 | Interruption detection | Toasts, banners, popups, cookie/paywall classification | 2.4 |
| WP-2.10 | Protected-content handling | Detect uncapturable/DRM, degrade with a clear message | 1.3 |
| WP-2.11 | Region-of-interest recapture | Dirty-rect + motion-triggered re-analysis | 1.4, 2.6 |
| WP-2.12 | Perception benchmarks | mAP, CER, latency per OS/EP | 0.6 |

### Exit criterion

- Detection mAP ≥ 0.80 (target 0.85) on the fixture set, all three OSes.
- OCR CER < 8% Latin scripts, < 15% CJK, at 1080p.
- Merge produces a graph with ≥ 95% of interactable elements correctly matched to the a11y tree where one exists.
- End-to-end perception p95 < 250 ms at 1080p on the reference machines.
- No frame leaves the process (asserted by a test that hooks the network).

### Rollback

If the detector underperforms on desktop (vs. web), we fine-tune on our fixture set. If int8 accuracy drops below target, we use fp16 for detection and int8 only for OCR. If a model has no ONNX path, we invoke ADR-014 and add a sidecar with a typed protocol.

---

## Phase 3 — Understanding (weeks 15–20)

**Goal:** turn a `ScreenGraph` into plain, chunked, correctly-leveled content — with a hard meaning-preservation contract.

### Work packages

| WP | Title | Deliverable | Depends on |
| --- | --- | --- | --- |
| WP-3.1 | Rules engine | Deterministic role/text/reflow rules, versioned, testable | 2.5 |
| WP-3.2 | Jargon + acronyms | Open dictionary, per-locale, user-addable | 3.1 |
| WP-3.3 | Number/date/unit normalization | Locale-aware, speakable forms | 3.1 |
| WP-3.4 | Simplification model | int4 instruct model, JSON contract, cache | 2.2 |
| WP-3.5 | Meaning-preservation checker | Entities, numbers, negation, comparatives | 3.4 |
| WP-3.6 | Summarization | Extractive + abstractive, chunked, key points | 3.4 |
| WP-3.7 | Chunking | Sentences, clauses, lists, headings, tables | 3.1 |
| WP-3.8 | Icon semantics | "what this icon does" from caption + context | 2.7 |
| WP-3.9 | "What will this do?" | Destructive-action preview, conservative list | 3.1 |
| WP-3.10 | Sensitive-content guard | Financial/medical/legal detection + warning + original | 3.1 |
| WP-3.11 | Cache | Content-hash + version key, LRU, encryption | 3.4 |
| WP-3.12 | Language detection | FastText-style lid, per-block | 3.1 |

### Exit criterion

- Simplification meaning-preservation score ≥ 4/5 on a 200-item human-rated set.
- No regression in named entities, numbers, or negation above 1% (automated).
- 100% of sensitive-content cases show the warning and the original.
- All understanding features work offline (network-kill-switch test).
- p95 simplify latency < 1.5 s for a 200-word paragraph on CPU.

### Rollback

If the local model cannot hit the meaning-preservation bar, we ship rules-only simplification first (deterministic, safer) and keep the model behind a "beta" flag. We never ship a simplifier that fails the checker silently.

---

## Phase 4 — Adaptation (weeks 21–27)

**Goal:** a rules-driven engine that turns a profile + graph into an `OverlayPlan`, with per-app memory.

### Work packages

| WP | Title | Deliverable | Depends on |
| --- | --- | --- | --- |
| WP-4.1 | Profile graph v1 | `focus`, `read`, `simple`, `calm`, custom; live preview | 1.2, 3.1 |
| WP-4.2 | Rules engine v1 | Forward chaining, priority, conflict resolution, user toggles | 3.1 |
| WP-4.3 | Dimming | Region-aware, except-list, opacity, reduced-motion | 4.2 |
| WP-4.4 | Icon grid | Tile layout, symbols, labels, search | 4.2 |
| WP-4.5 | Reflow | Font, size, spacing, alignment, line length, contrast | 4.2 |
| WP-4.6 | Ruler + lens | Reading ruler, focus lens, line highlight | 4.2 |
| WP-4.7 | Stop motion | Freeze carousels, pause autoplay, disable animations | 2.6, 4.2 |
| WP-4.8 | Interruption inbox | Collect, batch, quiet hours | 2.9, 4.2 |
| WP-4.9 | One-step mode | Step extraction, progress, visible answers | 4.2 |
| WP-4.10 | Confirm + undo | Destructive confirm, undo where possible | 3.9, 4.2 |
| WP-4.11 | Layout solver | Priority-based, no overlap, reading order, animated | 4.2 |
| WP-4.12 | Plan cache | Per app/window/content hash, invalidation | 4.2 |
| WP-4.13 | Per-app layouts | Remember user choices per app | 1.10, 4.12 |
| WP-4.14 | Provenance panel | "Why is this changed?" with rule references | 4.2 |
| WP-4.15 | Reduced-motion + contrast | Honor system settings | 4.2 |

### Exit criterion

- All four profiles produce sensible plans on the fixture set (human-rated ≥ 4/5).
- Plan build p95 < 50 ms after cache warm.
- No overlap or clipping in the layout solver under property-based fuzzing (10k cases).
- Every plan action has a rationale string visible in the provenance panel.
- A user can disable any rule and see the effect immediately.

### Rollback

If the layout solver is unstable, we ship a fixed-layout mode (one panel, one column) and iterate. If per-app memory causes privacy concerns, we default it to off and require explicit opt-in.

---

## Phase 5 — Rendering and interaction (weeks 28–34)

**Goal:** the overlay is pleasant, fast, accessible, and safe to interact with.

### Work packages

| WP | Title | Deliverable | Depends on |
| --- | --- | --- | --- |
| WP-5.1 | Overlay design system | Tokens, components, dark/light, reduced-motion | 0.8 |
| WP-5.2 | In-place renderer | Dim/grid/ruler/lens/highlight layers | 4.3–4.6 |
| WP-5.3 | Simplified view | Separate window, clean layout, focus handoff | 4.5, 4.9 |
| WP-5.4 | Hover + click routing | Hit-testing, click-through, focus handback | 5.2 |
| WP-5.5 | Keyboard operation | Full overlay control, visible focus, no traps | 5.2 |
| WP-5.6 | Switch access | Scanning, single-button, dwell | 5.2 |
| WP-5.7 | Large-target mode | ≥ 60 px targets, spacing | 5.2 |
| WP-5.8 | Animations | ≤ 150 ms, reduced-motion off, no parallax | 5.2 |
| WP-5.9 | Action consent | Per-session opt-in, confirmation, undo | 5.4 |
| WP-5.10 | Panic key behavior | Hide, stop capture, mute, restore | 1.8 |
| WP-5.11 | Provenance panel UI | Rationale list, show original | 4.14 |
| WP-5.12 | Overlay a11y audit | axe + platform audit + screen reader pass | 5.2 |
| WP-5.13 | Multi-display + DPI | Mixed DPI, 4K/5K, scaling | 1.7 |

### Exit criterion

- Overlay interactive p95 < 100 ms from input to visual response.
- 100% keyboard operability; no keyboard traps (automated + manual).
- Contrast ≥ 4.5:1; targets ≥ 60 px in large mode.
- Screen reader pass: VoiceOver, NVDA, Orca can operate the overlay.
- Zero crashes in a 4-hour dogfood session across 10 real apps.

### Rollback

If click-through is unreliable on Windows, we use a topmost overlay and a "hand focus" button. If the simplified view is too heavy, we ship in-place only and revisit.

---

## Phase 6 — Speech and voice (weeks 35–39)

**Goal:** hover-to-read, read-selection, read-from-here, and voice control, all offline.

### Work packages

| WP | Title | Deliverable | Depends on |
| --- | --- | --- | --- |
| WP-6.1 | TTS abstraction | Piper + espeak fallback, voice manager | 5.1 |
| WP-6.2 | Hover-to-read | Karaoke highlighting, word/line/paragraph | 5.2 |
| WP-6.3 | Read selection | Selection detection, read, stop | 5.2 |
| WP-6.4 | Read from here | Sequential reading with position memory | 6.2 |
| WP-6.5 | Voice control | whisper.cpp keywords, command grammar | 5.5 |
| WP-6.6 | Audio focus | Duck/pause screen readers, mix, mute | 6.1 |
| WP-6.7 | Rate/pitch/voice | Per-profile defaults, preview | 6.1 |
| WP-6.8 | Read our UI | All overlay text is speakable | 5.12 |

### Exit criterion

- Hover-to-speak p95 < 250 ms end-to-end.
- TTS works offline; espeak fallback < 50 ms.
- Voice commands ≥ 95% accuracy on a 200-utterance set in a quiet room, ≥ 85% noisy.
- No audio focus conflict with VoiceOver/NVDA/Orca in a 1-hour test.
- Karaoke highlight aligns within ±1 word on a 500-word sample.

### Rollback

If whisper.cpp is too heavy, we ship a smaller keyword spotter (e.g., a 10M-param model) and drop free-form voice. If Piper voices are unacceptable, we ship espeak first and treat Piper as an optional download.

---

## Phase 7 — Personalization (weeks 40–44)

**Goal:** the overlay learns the user's preferences and remembers app layouts, locally.

### Work packages

| WP | Title | Deliverable | Depends on |
| --- | --- | --- | --- |
| WP-7.1 | Profile editor | Live preview, import/export, reset | 4.1 |
| WP-7.2 | Learned simplifications | Accept/reject feedback, on-device adaptation | 3.4 |
| WP-7.3 | App layout memory | Per-app/window plans, migration | 4.13 |
| WP-7.4 | Memory aids | Steps visible, answers visible, next-up | 4.9 |
| WP-7.5 | Backup/restore | Encrypted export, versioned | 1.10 |
| WP-7.6 | Optional sync | E2E encrypted, off by default | 7.5 |
| WP-7.7 | Multi-user | Fast user switching, per-user stores | 1.10 |

### Exit criterion

- A returning user's profile and app layouts survive reinstall (backup/restore test).
- Learned simplifications improve acceptance rate by ≥ 10% over 2 weeks in dogfood.
- Sync is E2E encrypted and disabled by default; a network capture shows no plaintext.
- Per-user stores are isolated (cross-user read test fails as expected).

### Rollback

If sync is a security liability, we ship local-only and revisit. If learning drifts, we cap the adaptation rate and require explicit accept/reject.

---

## Phase 8 — Quality and hardening (weeks 45–50)

**Goal:** be able to ship to strangers without shame.

### Work packages

| WP | Title | Deliverable | Depends on |
| --- | --- | --- | --- |
| WP-8.1 | Test blitz | ≥ 80% unit coverage on core, 100% on rules/merge | all |
| WP-8.2 | Property + fuzz | Merge, layout, cache, config | all |
| WP-8.3 | E2E suite | Playwright + scripted app harness | 5.13 |
| WP-8.4 | Perf suite | 3 reference machines, budgets, regression gates | 0.6 |
| WP-8.5 | A11y audit | External audit + screen reader + switch user | 5.12 |
| WP-8.6 | Security audit | External penetration test, dependency audit | 0.7 |
| WP-8.7 | Privacy audit | Data inventory, network capture, memory scan | 0.7 |
| WP-8.8 | Chaos | Permission loss, corrupt cache, huge screens, RDP | 5.13 |
| WP-8.9 | Documentation | User docs, admin docs, developer docs | all |
| WP-8.10 | Dogfood | 20 users × 2 weeks, feedback loop | all |

### Exit criterion

- No critical or high CVEs; all audit findings closed or explicitly accepted.
- ≥ 80% coverage on core crates; 100% on rules, merge, and privacy guards.
- Perf budgets met on all three reference machines.
- External a11y audit: no blocker findings.
- Dogfood: ≥ 70% of participants say they would keep using it; all critical bugs fixed.

### Rollback

If an audit finds a privacy or security blocker, we delay the release and fix it. No exceptions. If perf budgets fail, we cut features (start with the LLM simplifier) rather than ship a hot, laggy app.

---

## Phase 9 — Beta and release (weeks 51–58)

**Goal:** a signed, reproducible, updatable 1.0.

### Work packages

| WP | Title | Deliverable | Depends on |
| --- | --- | --- | --- |
| WP-9.1 | Installers | `.dmg` (notarized), `.msi` (signed), `.AppImage`/`.deb`/Flatpak | 8.x |
| WP-9.2 | Updater | Signed, rollback, changelog | 1.1 |
| WP-9.3 | Beta program | 1,000 users, feedback triage, NDA-free | 8.10 |
| WP-9.4 | VPAT + statement | Generated from the a11y mapping | 0.7 |
| WP-9.5 | Pricing + license | Free core, Pro, enterprise; terms | 7.6 |
| WP-9.6 | Support | Docs, community, paid support SLAs | 9.1 |
| WP-9.7 | 1.0 release | Tag, release notes, blog, advisory sign-off | all |
| WP-9.8 | Post-launch | Crash analytics (opt-in), SLO dashboards | 1.11 |

### Exit criterion

- 1.0 installs and runs on a clean machine on all three OSes without a dev environment.
- Auto-update works and rolls back on a corrupted update.
- Advisory panel signs off on the release.
- Support load is < 1 ticket per 100 users per week at launch.

---

## Phase 10 — Ecosystem (post-1.0)

- **Browser extension** — a thin client that uses the same rules/profile engine inside the page, for users who only need web.
- **Mobile companion** — profile sync and remote read-aloud (no overlay on mobile).
- **SDK** — let apps embed a "simplify this" component.
- **Enterprise console** — MDM, fleet profiles, aggregate (non-content) metrics.
- **Marketplace** — community profiles, jargon dictionaries, voices.
- **Research** — publish the fixture dataset, benchmarks, and usability results.

---

## Cross-cutting workstreams

These run every phase:

1. **Privacy and security** — data inventory updated, threat model revised, kill-switch tested.
2. **Accessibility of our own UI** — audit each release candidate.
3. **Advisory panel** — monthly demo, quarterly gate.
4. **Performance** — budget checked in CI on every PR.
5. **Docs** — every feature ships with user docs and a "show original" path.
6. **Licensing** — `cargo-deny` and `reuse` in CI; no new dependencies without a license check.

---

## Risk register (live)

| Risk | Phase | Trigger | Response |
| --- | --- | --- | --- |
| Permission friction kills activation | 1 | Onboarding drop-off > 40% | Per-feature permissions, better copy, manual import |
| Detector misses desktop UI | 2 | mAP < 0.70 | Fine-tune on fixtures; fall back to a11y-only where available |
| Simplification hallucinates | 3 | Meaning score < 4/5 | Rules-only mode; human review |
| Overlay breaks apps | 5 | > 5% dogfood reports | Read-only default; disable the offending rule |
| Perf budget blown | 5 | p95 > 500 ms | Drop to 1 fps analysis; region-of-interest; smaller model |
| Privacy incident | any | Any content in a log/crash report | Incident response plan; fix; disclose |
| Advisory panel rejects a release | any | Gate fails | Do not ship; re-plan |

---

## Definition of ready (for a WP)

- A goal and a deliverable.
- At least one measurable exit criterion.
- Dependencies resolved or explicitly gated.
- An owner and an estimate.
- A privacy/permission note.
- A rollback plan.
- A test plan.
