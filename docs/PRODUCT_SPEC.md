# Product Specification

> Audience: designers, engineers, and the advisory panel. Every requirement has an ID (`PRD-xx`) that appears in the backlog.

---

## 1. Vision

A desktop layer that makes any application usable for people whose attention, memory, reading, or processing speed differs from the assumed default — and that makes long or dense sessions better for everyone.

## 2. Non-goals

- We do not make apps WCAG-conformant.
- We do not replace screen readers, switch access, or dictation tools.
- We do not perform actions autonomously.
- We do not ship a cloud service that processes screen content.
- We do not support kernel-level injection, rootkits, or anything that breaks platform security.
- We do not simplify medical, legal, or financial content automatically without a warning and the original.

---

## 3. Personas

| ID | Persona | Core need | Default profile |
| --- | --- | --- | --- |
| P1 | Yuki, ADHD | focus, no interruptions, short paths | `focus` |
| P2 | Tal, dyslexia | spacing, ruler, read-aloud, plain text | `read` |
| P3 | George, Down syndrome | symbols, one step at a time, no memory | `simple` |
| P4 | Maria, memory loss | steps visible, undo, confirmations | `simple` |
| P5 | Kwame, TBI | short paths, no interruptions, feedback | `focus` |
| P6 | Amy, autistic | literal text, predictable layout, no motion | `calm` |
| P7 | Alison, aging | familiar patterns, patience, large targets | `simple` |
| P8 | ESL reader | plain language, definitions, slow read-aloud | `read` |

Profiles are presets of a shared settings graph, not silos. Users mix and override.

---

## 4. Feature set

### F1. Perception (`PRD-01..14`)

- `PRD-01` Detect displays and windows; capture the active window or full display.
- `PRD-02` Detect interactable regions (buttons, menus, fields, tabs, links, icons).
- `PRD-03` OCR all text, with reading order and confidence.
- `PRD-04` Classify element roles from vision + a11y tree (button, link, text field, checkbox, tab, menu, dialog, banner, toast, carousel, video).
- `PRD-05` Merge a11y tree and vision into one element graph with provenance per property.
- `PRD-06` Detect layout regions (header, nav, main, aside, footer, modal, floating).
- `PRD-07` Detect motion and media (animations, carousels, autoplay, countdowns).
- `PRD-08` Detect reading level per text block (Flesch-Kincaid, SMOG, syllable count, word frequency).
- `PRD-09` Detect icons and generate plain-language labels for them.
- `PRD-10` Detect dialogs and interruptions (toasts, popups, cookie banners, paywalls).
- `PRD-11` Track element stability across frames (to avoid flicker and to support scroll).
- `PRD-12` Detect app identity and window title for per-app layouts.
- `PRD-13` Detect protected/uncapturable content and degrade gracefully.
- `PRD-14` Offline, on-device, no network for perception.

### F2. Understanding (`PRD-15..22`)

- `PRD-15` Plain-language rewrite of dense text (local model + rules), with meaning-preservation checks.
- `PRD-16` Summary of long documents and pages, chunked into sections.
- `PRD-17` Jargon dictionary + acronym expansion + number/date/unit formatting.
- `PRD-18` Icon semantics ("this gear icon opens settings") from caption model + context.
- `PRD-19` "What will this do?" preview for destructive or irreversible controls.
- `PRD-20` Reading-level target selection (e.g., grade 6, grade 9, keep original).
- `PRD-21` Language detection and bilingual support.
- `PRD-22` Cache results per content hash; invalidate on change.

### F3. Adaptation (`PRD-23..34`)

- `PRD-23` Profiles: focus, read, simple, calm, custom.
- `PRD-24` Icon grid: replace dense menus/toolbars with large labelled tiles.
- `PRD-25` Distraction dimming: everything except the active region is dimmed.
- `PRD-26` Focus lens / reading ruler / line highlight.
- `PRD-27` Text reflow: chunking, spacing, alignment, line length, font, size, contrast.
- `PRD-28` Stop motion: freeze carousels, pause autoplay, disable animations.
- `PRD-29` Interruptions: collapse toasts/banners into a quiet inbox; batch notifications.
- `PRD-30` One-step mode: break multi-step flows into visible, numbered steps.
- `PRD-31` Memory aids: keep entered values visible, show progress, show "what's next".
- `PRD-32` Undo and confirmation: warn before irreversible actions; offer undo where possible.
- `PRD-33` Simplified view: re-present the app in a clean, large-target layout.
- `PRD-34` Per-app layouts: remember the user's preferred simplification per app/window.

### F4. Reading and speech (`PRD-35..42`)

- `PRD-35` Hover-to-read with karaoke highlighting.
- `PRD-36` Read selection.
- `PRD-37` Read from here.
- `PRD-38` Voice control (keywords: read, stop, next, simplify, focus, dim).
- `PRD-39` Adjustable rate, pitch, voice, and highlight color.
- `PRD-40` Audio focus cooperation with screen readers.
- `PRD-41` Offline TTS with instant espeak fallback.
- `PRD-42` Read-aloud for our own UI.

### F5. Input and navigation (`PRD-43..50`)

- `PRD-43` Global shortcuts for every core action.
- `PRD-44` Keyboard-only operation of the entire overlay.
- `PRD-45` Mouse hover and click routing.
- `PRD-46` Switch access and single-button scanning.
- `PRD-47` Large-target mode (all targets ≥ 60×60 px).
- `PRD-48` Hand focus back to the real app (click-through to the real control, with consent).
- `PRD-49` Voice commands as an alternative to shortcuts.
- `PRD-50` Onboarding: guided permission setup, profile picker, 2-minute tutorial.

### F6. Personalization and data (`PRD-51..60`)

- `PRD-51` Encrypted local store (SQLite + SQLCipher/stronghold).
- `PRD-52` Profile editor with live preview.
- `PRD-53` Per-app layout memory.
- `PRD-54` Learned simplifications (user accepts/rejects; we adapt).
- `PRD-55` Import/export profiles (portable, shareable).
- `PRD-56` Optional encrypted sync (off by default).
- `PRD-57` Usage stats, local only, opt-in export.
- `PRD-58` Telemetry: no content, no identifiers, off by default.
- `PRD-59` Backup and restore.
- `PRD-60` Multi-user / fast user switching support.

### F7. Trust and safety (`PRD-61..70`)

- `PRD-61` Never log screen content.
- `PRD-62` Memory-zeroing for frame buffers.
- `PRD-63` Network kill-switch (compile-time and runtime).
- `PRD-64` Permission-per-feature with clear rationale.
- `PRD-65` "Why is this blurred / hidden / simplified?" provenance panel.
- `PRD-66` Panic key: hide overlay, stop capture, mute TTS, in one keystroke.
- `PRD-67` Sensitive-content detector: warn before simplifying financial/medical/legal content.
- `PRD-68` Update signing and reproducible builds.
- `PRD-69` VPAT / accessibility statement generator.
- `PRD-70` Advisory panel review gate for releases.

---

## 5. Core user journeys

### J1. First run (P2, Tal)

1. Download, install, launch.
2. Guided permissions: screen recording, accessibility, (optional) microphone. Each explains why.
3. Profile picker: "I have dyslexia" → `read`.
4. 2-minute tutorial on a sample screen.
5. Overlay appears on the desktop; Tal hovers a paragraph and hears it.
6. He opens a dense settings window; CurbCut offers an icon grid. He accepts.
7. Done.

### J2. Focus rescue (P1, Yuki)

1. Yuki is in a cluttered app with a carousel and three toasts.
2. She hits `Ctrl+Shift+F`.
3. CurbCut dims everything but the main region, freezes the carousel, files the toasts into an inbox.
4. She works. She hits `Ctrl+Shift+I` to review the interruption inbox.
5. She hits `Ctrl+Shift+F` again to restore.

### J3. One step at a time (P3, George)

1. George opens a 9-step form.
2. CurbCut offers one-step mode.
3. Only step 1 is visible, with a big "Next" tile and a visible progress indicator.
4. Entered values stay visible in a "Your answers" panel.
5. Each step has "Back" and "Help".

### J4. Read aloud (P8, ESL)

1. Ana selects a dense policy paragraph.
2. She hits `Ctrl+Shift+R`.
3. CurbCut reads it at grade-8 level with karaoke highlighting.
4. She clicks a word to hear it and see a plain definition.
5. She exports the simplified version.

---

## 6. UX principles

1. **Nothing surprises.** Every change is reversible and announced.
2. **Show the original.** Every simplification has a "show original" affordance.
3. **One idea at a time.** Never stack two modal prompts.
4. **Big and literal.** Targets ≥ 60 px, plain labels, no metaphors.
5. **Quiet by default.** The overlay is invisible until asked.
6. **Explain yourself.** Every hidden or changed element can be justified.
7. **Fast and patient.** Instant response, no timeouts, no "are you still there?"
8. **Consistent.** Same shortcuts, same panels, same words everywhere.

---

## 7. Accessibility of our own UI

CurbCut must be usable by the same people it serves, and by screen reader users:

- All controls expose Name/Role/Value/State through the platform a11y API.
- Full keyboard operation, visible focus, no keyboard traps.
- Respects `prefers-reduced-motion`, `prefers-contrast`, and system font scaling.
- Contrast ≥ 4.5:1 (AA), ≥ 7:1 for small text where possible.
- No information conveyed by color alone.
- Captions/transcripts for any audio we produce.
- Tested with VoiceOver, NVDA, Orca, and at least one switch user.

---

## 8. Definition of done

A feature is done when:

- It works offline.
- It has unit tests and at least one integration test.
- It passes the benchmark suite with no regression > 5%.
- It is reviewed by at least one engineer and one designer.
- It has been tried by at least one advisory panel member where it touches the overlay.
- Its privacy impact is documented.
- Its docs and its "show original" behavior are in place.
