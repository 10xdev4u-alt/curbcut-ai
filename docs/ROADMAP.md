# Roadmap

> Dates are targets, not promises. Each phase gate is a decision point. "Weeks" are elapsed from kickoff, not parallel.

---

## Timeline

```
W1 ──────── W14 ──────── W27 ──────── W39 ──────── W50 ──────── W58 ────────►
  Phase 0-1     Phase 2-3      Phase 4-5      Phase 6-7      Phase 8        Phase 9
  Foundations   Perception     Adaptation     Speech         Hardening      Release
                Understanding  Rendering      Personalization
```

| Phase | Weeks | Goal | Gate |
| --- | --- | --- | --- |
| 0 | 1–3 | Foundations | CI green, dataset, panel, threat model |
| 1 | 4–7 | Platform core | Capture + overlay + store on 3 OSes |
| 2 | 8–14 | Perception | mAP ≥ 0.80, CER < 8%, p95 < 250 ms |
| 3 | 15–20 | Understanding | Meaning score ≥ 4/5, offline |
| 4 | 21–27 | Adaptation | 4 profiles, no overlap, rationale |
| 5 | 28–34 | Rendering | p95 < 100 ms, keyboard, a11y pass |
| 6 | 35–39 | Speech | Hover-to-speak < 250 ms, voice control |
| 7 | 40–44 | Personalization | Backup/restore, learned simplifications |
| 8 | 45–50 | Hardening | Audits, perf, dogfood |
| 9 | 51–58 | Release | Signed 1.0, beta, VPAT |
| 10 | post | Ecosystem | Extension, mobile, SDK, enterprise |

---

## Milestones

| ID | Milestone | Criteria |
| --- | --- | --- |
| M0 | Repo + CI + panel | Phase 0 exit |
| M1 | "Hello, overlay" | Transparent overlay on 3 OSes, panic key |
| M2 | "It sees" | First `ScreenGraph` from a real app, CLI dump |
| M3 | "It simplifies" | First end-to-end simplification with a meaning check |
| M4 | "It adapts" | First icon grid + dimming on a real app |
| M5 | "It speaks" | Hover-to-read in under 250 ms |
| M6 | "It remembers" | Per-app layouts and backup/restore |
| M7 | "It is safe" | External security + a11y + privacy audits clean |
| M8 | 1.0 | Signed release, advisory sign-off |

---

## Now / next / later

**Now (Phase 0–1):** repo, CI, fixture dataset, panel, capture + overlay skeleton.

**Next (Phase 2–4):** perception, understanding, adaptation. This is where the product is won.

**Later (Phase 5–7):** rendering polish, speech, personalization.

**After 1.0:** browser extension, mobile companion, SDK, enterprise console, open benchmark.

---

## What we will not schedule

- **A "compliance" feature.** We do not sell conformance.
- **An autonomous agent.** Out of scope by design.
- **A social network.** No.
- **A mobile overlay.** The model does not fit mobile OS constraints.
- **A cloud summarizer.** Privacy and latency both fail.

---

## Gate review format

At each phase gate, the team answers in writing:

1. What did we ship?
2. What did we measure? (bench numbers, not adjectives)
3. What did the advisory panel say?
4. What did we learn?
5. What are we cutting or changing?
6. Do we proceed, iterate, or stop?

The answers are committed to `docs/gates/phase-N.md` and tagged.
