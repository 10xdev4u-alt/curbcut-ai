# CurbCut AI

**Real-time cognitive accessibility overlay for any desktop app.**

CurbCut AI watches the screen with an on-device vision model, understands what is on it, and re-presents it in a way that works for ADHD, dyslexia, low literacy, brain injury, aging, and non-native readers — without sending a single pixel to the cloud.

It is not a screen reader. It is not a browser extension. It is a **curb cut**: a layer that helps the people who need it and makes things better for everyone.

---

## What it does

- **See any app.** Screen capture + on-device detection finds buttons, menus, fields, icons, and text in native apps, Electron apps, terminal UIs, games, and remote desktops — not just web pages.
- **Use the accessibility tree when it can.** Where macOS AX, Windows UI Automation, or Linux AT-SPI exposes semantics, CurbCut uses them as a fast, exact signal and falls back to vision where the tree is silent.
- **Simplify, don't shrink.** Dense menus become big icon grids with plain labels. Walls of text become short chunks with a summary. Jargon becomes plain language.
- **Protect attention.** Dim everything except the active task. Stop carousels and autoplay. Collapse notifications. Add a reading ruler and a focus lens.
- **Read aloud.** Hover any element and hear it. Follow along with karaoke highlighting. Control it all by voice or keyboard.
- **Remember for you.** Keep the steps of a task visible. Don't rely on memory. Offer undo and confirmation for irreversible actions.
- **Stay yours.** All inference is local. Profiles, app layouts, and preferences live in an encrypted local store. No account. No telemetry by default.

---

## Status

**Planning / pre-alpha.** This repository currently contains the research, architecture, engineering plan, governance files, and a **124-issue / 124-PR / 329-commit** backlog with 246 review threads. No product code has shipped yet.

See [`docs/ROADMAP.md`](docs/ROADMAP.md) for the plan and [`backlog/`](backlog/) for the full backlog.

---

## Why this exists

Accessibility tooling is overwhelmingly blind-centric. Screen readers are excellent and necessary, but they solve one class of barrier. Cognitive accessibility — attention, memory, reading, processing speed, executive function — is massively underserved, harder to diagnose, and affects far more people than the blind community.

The W3C's own cognitive accessibility guidance is **supplemental**, not normative, and WCAG 2.2 explicitly says it does not address every user need for people with cognitive and learning disabilities. That gap is the opportunity.

Meanwhile, the regulatory floor is rising: the EU European Accessibility Act applied from **28 June 2025**, EN 301 549 is the presumed-conformity standard, and the US ADA / Section 508 / state laws keep expanding. But compliance is a floor, not a product. CurbCut targets **usable**, not merely conformant.

---

## The honest framing

CurbCut is **not** an "accessibility overlay" in the accessiBe sense, and the difference is load-bearing:

| accessiBe-style overlay | CurbCut AI |
| --- | --- |
| Runs in the page, rewrites DOM | Runs at the OS surface, reads pixels + a11y tree |
| Claims to make a site WCAG-compliant | Claims to make a session more usable, never compliant |
| Targets web only | Targets every app |
| Sells to site owners to dodge remediation | Sells to users and employers who want access |
| Opaque, marketing-driven | Open source, auditable, local-first |
| FTC ordered $1M over deceptive claims (Jan 2025) | Publishes its limits and its test results |

We will not claim compliance. We will claim measured outcomes for named personas, and we will publish the failures too.

---

## Who it is for

Primary personas (from W3C COGA, extended):

- **Yuki** — ADHD, loses focus, overwhelmed by dense pages and carousels.
- **Tal** — dyslexia, needs spacing, ruler, read-aloud, plain language.
- **George** — Down syndrome, needs symbols, one step at a time, no memory load.
- **Maria** — memory loss, needs steps visible, undo, confirmation.
- **Kwame** — TBI, needs short critical paths, no interruptions, clear feedback.
- **Amy** — autistic, needs literal language, predictable layout, no surprise motion.
- **Alison** — aging, mild cognitive impairment, needs familiar patterns and patience.

Secondary (the curb cut): ESL readers, tired clinicians, students, traders, anyone in a noisy room or on a small screen.

---

## Repository map

```
README.md                  This file
docs/
  RESEARCH.md              Deep research: problem, prior art, market, tech, risks
  PRODUCT_SPEC.md          Features, personas, user journeys, UX principles
  ARCHITECTURE.md          System architecture, crates, data flow, event model
  ENGINEERING_PLAN.md      Phase-by-phase plan with exit criteria
  ADAPTERS.md              Per-OS capture / a11y / input / TTS details
  MODELS.md                Model zoo, budgets, quantization, fallbacks
  INNOVATIONS.md           Creative extensions and differentiators
  THREAT_MODEL.md          Security and privacy threat model
  A11Y_MAPPING.md          WCAG 2.2 + COGA mapping and honest limits
  ROLE_MAPPING.md          Platform role/state compatibility tables (generated)
  REDACTION.md             Redaction layer, canary fixture, no-content CI test
  RULES.md                 Deterministic rule layer: ids, thresholds, tests
  CLI.md                   Headless CLI: commands, exit codes, redacted dump
  VISION.md                Vision pipeline: scheduler, stages, privacy
  PROFILE_GRAPH.md         Profile graph: 40-setting registry, preview, validation
  LAYOUT.md                Layout solver: tile packing and reflow, invariants, fuzz
  DIMMING.md               Dimming: except-regions, scrim plan, reduced motion
  GRID.md                  Icon grid: tile layout, label policy, search
  SHORTCUTS.md             Global shortcut catalog: actions, defaults, rebinding
  PLAN_CACHE.md            Plan cache: bounded LRU plans and opt-in per-app memory
  CONFIRM.md               Confirmation and undo: consequence, outcome, action log
  HOVER.md                 Pointer tracking: dwell, move threshold, Wayland limit
  CLICKS.md                Click routing: consume or hand off, focus, no double-click
  PROVENANCE.md            Provenance panel: rule links, originals, log-safe hashes
  FOCUS.md                 Reading ruler and focus lens: lead, dead zone, reduced motion
  READING_LEVEL.md         Reading scores: Flesch, FK, SMOG, ARI, script check, cache
  JARGON.md                Jargon dictionary and acronym expansion: sources, locales, overrides
  NORMALIZE.md             Numbers, dates, and units spoken: money, unambiguous dates, unit systems
  SPEECH.md                TTS engines, voices, fallback chain, and audio focus
  VOICE_COMMANDS.md        Voice command grammar, help, and push-to-talk
  SENSITIVE_CONTENT.md     Sensitive-content guard: financial, medical, legal
  DESTRUCTIVE_ACTIONS.md   Destructive-action catalog, preview, and confirmation
  MERGE.md                 A11y + vision merge: IoU matching, provenance, conflicts
  STABILITY.md             StableIds across frames, scroll compensation, re-analysis
  MEANING.md               Meaning-preservation checker: entities, numbers, negation
  ETHICS.md                The never list, honest-claim standard, panel authority
  ROADMAP.md               Timeline and milestones
backlog/
  data_issues.py           Source data: 15 epics, 124 issues
  data_prs.py              Source data: 124 PRs (derived from issues)
  data_commits.py          Source data: 329 commits (root + PR commits)
  EPICS.md                 Epics and definitions of done
  ISSUES.md                124 issues with acceptance criteria
  PRS.md                   124 PR specs with review threads
  COMMITS.md               329-commit sequence
  METRICS.md               Backlog statistics
scripts/
  render_backlog.py        Regenerates the backlog documents from source data
  new_issue.py             Scaffold a correctly-labelled issue
  publish_issues.py        Publish the backlog to GitHub and record the CC-># map
  scaffold_workspace.py    Regenerates the 14-crate workspace skeleton
.github/
  ISSUE_TEMPLATE/          Bug, feature, and engineering-task templates
  PULL_REQUEST_TEMPLATE.md PR checklist (privacy, security, a11y, rollback)
  CODEOWNERS               Per-path human ownership
  workflows/ci.yml         4-target matrix, frontend, backlog drift, scaffold drift
CONTRIBUTING.md            How to contribute, review culture, setup
CODE_OF_CONDUCT.md         Contributor Covenant 2.1
SECURITY.md                Disclosure policy and security model
LICENSE                    AGPL-3.0
```

---

## Principles

1. **Local first.** No cloud call is required for any core feature. If a feature needs the network, it is opt-in, off by default, and degrades gracefully.
2. **Pixels are sensitive.** Screen contents are the most private data a machine holds. Treat every frame as a secret. Process in memory, encrypt at rest, never log content.
3. **Nothing moves without consent.** The overlay is a guest. It never clicks, types, submits, or dismisses anything unless the user asked for that exact action in that exact session.
4. **Measure, don't marketed.** Every claim in the README must be reproducible from a benchmark or a usability session in this repo.
5. **Design with users, not for them.** Paid advisory panel of users with cognitive and learning disabilities. Their feedback is a release gate, not a blog post.
6. **Small, sharp, composable.** One process does one thing. Crates are small. The overlay is a plugin, not a monolith.

---

## License

TBD — likely AGPL-3.0 for the core (to prevent proprietary relicensing of the accessibility layer) with a commercial exception for OEM/enterprise embedding. See [`docs/RESEARCH.md`](docs/RESEARCH.md#licensing) for the analysis.

---

## Contact

Do not open issues for support. For accessibility feedback, see [`CONTRIBUTING.md`](CONTRIBUTING.md). For security, see [`SECURITY.md`](SECURITY.md).
