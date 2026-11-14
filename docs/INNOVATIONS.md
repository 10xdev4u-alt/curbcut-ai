# Creative Extensions and Differentiators

> Twenty ideas that make CurbCut more than "a vision model plus a toolbar." Each has a hypothesis, a sketch, and a reason it might matter. Ranked by expected impact × feasibility. This is the sandbox; the roadmap picks the winners.

---

## Tier 1 — ship in v1 or v1.5

### I1. Provenance-aware overlay ("Why did it change?")

Every dimmed region, icon tile, and simplified paragraph carries a rationale: *"dimmed because the carousel is moving," "chunked because this paragraph is grade 14," "icon grid because this menu has 14 items."* Clicking shows the rule and the original.

**Why it matters:** trust. Users with cognitive disabilities are routinely patronized by software. Showing the reason turns the overlay from a black box into a tool they can audit and argue with.

**Feasibility:** high. We already have the rule engine; this is a UI surface over `Rationale`.

---

### I2. Reading ruler + focus lens as a first-class object

A movable ruler that isolates one, two, or three lines; a lens that magnifies and reflows a region; both follow the pointer with a slight lead and no jitter.

**Why it matters:** the single most requested dyslexia/ADHD tool, and existing implementations are browser-only and janky.

**Feasibility:** high. Pure rendering + pointer input.

---

### I3. Interruption inbox

Toasts, banners, cookie notices, chat pings, and "you are away" prompts are collected into a quiet inbox instead of appearing over the work. The user reviews them on their own schedule, batched, with a "reply later" action.

**Why it matters:** interruptions are the top ADHD barrier. COGA 4.6.1.

**Feasibility:** medium. Detection is heuristic; the UI is straightforward. The hard part is not missing a genuinely urgent interruption (e.g., a download finished). We never auto-dismiss; we auto-file.

---

### I4. One-step mode with visible answers

Multi-step forms become one step at a time, with a persistent "Your answers" panel and a progress indicator. Back is always available; nothing is hidden.

**Why it matters:** working memory. COGA 4.7, 4.4.9.

**Feasibility:** medium. Requires reliable step detection, which is app-specific. Start with heuristics (wizard roles, page dots) and per-app memory.

---

### I5. Icon grid with plain labels

Dense menus and toolbars become large tiles with a short plain label plus the original icon. A search box filters tiles. Tiles are ≥ 72 px, 2–3 columns.

**Why it matters:** recognition over recall. COGA 4.2.7, 1.3.3.

**Feasibility:** high for detection; medium for good labels (needs icon captioning + a11y fallback).

---

### I6. Stop-motion mode

Freeze carousels, pause autoplay video, disable CSS/JS animations, and hide countdown timers (or replace them with a calm static deadline). Respects `prefers-reduced-motion` and goes further.

**Why it matters:** COGA 4.9.1, WCAG 2.2.2. Autoplay is a top complaint from autistic and ADHD users.

**Feasibility:** medium. We can't always reach into the app's DOM (native apps), so we cover the region with a static snapshot and a "play" button. Honest limitation: the underlying motion continues; we just hide it.

---

### I7. Hover-to-read with karaoke

Hover any text and hear it at the profile rate, with word-level highlighting. Click a word for a definition and a slower repetition.

**Why it matters:** the core read-aloud loop; low effort for the user.

**Feasibility:** high with Piper + an off-screen render for highlight timing.

---

### I8. Profile graph, not presets

Four starter profiles are just saved graphs of ~40 settings. Users see every setting, can copy a profile, and can share it as a file. No "AI profile generator" black box.

**Why it matters:** cognitive accessibility is individual; presets are a starting point, not a destination.

**Feasibility:** high.

---

## Tier 2 — v2 candidates

### I9. Task maps

For a repeated task (expense report, booking, exam), CurbCut records the steps once and shows a persistent map: *Step 3 of 7 — "Upload receipt."* The map survives app restarts and is editable.

**Why it matters:** executive function and memory. Turns a procedure into an external scaffold.

**Feasibility:** medium. Requires safe recording of actions (with consent) and a robust step model. Privacy-sensitive; local-only.

---

### I10. "Read the room" — cognitive load meter

A private, on-device estimate of current load (element count, motion, interruption rate, time-on-task) shown as a calm 1–5 indicator. When it rises, CurbCut offers a simplification before the user asks.

**Why it matters:** many users cannot self-advocate in the moment. A gentle prompt is the product.

**Feasibility:** medium. The meter is a heuristic; the value is in the timing and the offer, not the number. Must never be gamified or shaming.

---

### I11. Plain-language contract

Every simplification is accompanied by a one-line "what changed and what didn't": *"Shortened sentences. Kept all numbers and warnings."* A "show original" toggle is always one click away.

**Why it matters:** meaning preservation is the whole ballgame for trust.

**Feasibility:** high, if the simplifier emits a structured diff.

---

### I12. Symbol library (ARASAAC-style)

For users who need symbols, replace or augment text with an open symbol set. Start with a small set for common actions (save, back, next, help, warning, money, time).

**Why it matters:** COGA George persona; low-literacy users.

**Feasibility:** medium. Licensing (ARASAAC is CC-BY-NC-SA; we may need a different set). Rendering and layout are easy.

---

### I13. Bilingual mode

Detect the language of a block and show a side-by-side or toggle with a second language, with read-aloud in both.

**Why it matters:** ESL readers are a huge underserved group; the "curb cut" persona.

**Feasibility:** medium. Translation can be local (small model) or omitted; even language detection + dictionary is valuable.

---

### I14. Focus sessions

A user starts a 25-minute focus session. CurbCut dims everything outside the active app, files interruptions, and shows a calm remaining-time bar. No notifications escape.

**Why it matters:** ADHD body-doubling and time-boxing, in the tool the user already has open.

**Feasibility:** high. Combines existing primitives.

---

### I15. App-specific "calm layouts"

Community-authored layouts for the worst offenders (a specific gov website, a specific EHR, a specific bank). A layout is a set of rules + selectors + a profile, versioned and reviewable.

**Why it matters:** the long tail of bad apps is the product. A marketplace of calm layouts is a moat.

**Feasibility:** medium. Needs a safe layout format and a review process. Start with first-party layouts for the top 20 apps.

---

## Tier 3 — research / moonshots

### I16. On-device "explain this screen"

A local VLM answers "what is on this screen and what can I do?" in one plain sentence. Not an agent — it never acts.

**Why it matters:** orientation for users who get lost in complex UIs.

**Feasibility:** medium-low. Needs a small VLM that runs fast enough; hallucination control is hard. Cache aggressively.

---

### I17. Cognitive accessibility score (private)

A local, private score for an app's cognitive load: element count, motion, reading level, interruption rate, steps to task. Never shared, never a public shaming metric.

**Why it matters:** gives users vocabulary and gives us a regression metric.

**Feasibility:** high. Pure measurement. The risk is misuse; keep it local and explain it.

---

### I18. Voice companion for forms

"Fill the form by voice" — the user says "name is Sam," CurbCut finds the name field and types it, with confirmation. Local ASR.

**Why it matters:** motor + cognitive + memory in one.

**Feasibility:** medium. Field matching is the hard part; start with explicit "click the field, then dictate."

---

### I19. Shared focus / body doubling

Two users (with consent) see each other's calm focus status and can send a single pre-written "you've got this" nudge. No chat, no content.

**Why it matters:** ADHD support often comes from other people. This is a privacy-preserving nudge, not a social network.

**Feasibility:** low-medium. Needs sync, safety, and abuse prevention. High risk; gate behind research.

---

### I20. Open cognitive accessibility dataset and benchmark

Publish the fixture dataset, the benchmark harness, and the usability protocol. Invite researchers and competitors to measure against us.

**Why it matters:** the field has no good public benchmark for cognitive accessibility. Being the yardstick is a moat and a public good.

**Feasibility:** high. We need the dataset anyway. Licensing and consent are the work.

---

## What we deliberately did not build

- **An agent that clicks for you.** It is a different product with different risks. We present; the user acts.
- **A public "accessibility score" for websites.** It would be a shaming tool and a legal target.
- **A cloud summarizer.** Privacy and latency both fail.
- **A "cure" or a diagnostic.** We are a tool, not a clinician.
- **A gamified focus app.** Points and streaks are extra cognitive load.
- **A browser-only extension.** It would be another Helperbird. The whole point is the whole desktop.

---

## How to pick

For each idea, score 1–5 on:

1. **Impact** for a named persona.
2. **Feasibility** with our stack.
3. **Moat** — is it hard to clone?
4. **Trust** — does it increase or decrease user trust?
5. **Reversibility** — can we ship it behind a flag and remove it?

The roadmap picks ideas with a total ≥ 18 and at least a 4 on impact and trust.
