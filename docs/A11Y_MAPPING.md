# Accessibility Mapping

> What WCAG 2.2 and W3C COGA ask for, what CurbCut does about it, and what we honestly cannot do. This is the source for the VPAT and the public limitations page.

---

## 1. Framing

CurbCut is an **alternative perceptual channel**, not a conformance fix. It does not change the app's markup, semantics, or code. It can therefore help a user complete tasks in an app that is not accessible, but it cannot make that app conform to WCAG. We say this everywhere.

Two distinct claims:

- **Usability claim:** for a named persona, on a fixed benchmark, task completion and time improve by X. This is our claim.
- **Conformance claim:** the app conforms to WCAG. This is **never** our claim.

---

## 2. WCAG 2.2 mapping

| Criterion | Level | CurbCut behavior | Status |
| --- | --- | --- | --- |
| 1.1.1 Non-text Content | A | Icon captioning + a11y names provide text alternatives for icons we detect. | Partial; only for detected icons |
| 1.3.1 Info and Relationships | A | The simplified view preserves headings, lists, and labels; we never flatten structure. | Partial; depends on detection |
| 1.3.2 Meaningful Sequence | A | Reading-order analysis drives the simplified view. | Partial |
| 1.3.3 Sensory Characteristics | A | We never rely on shape/color alone; labels are always textual. | Our UI: yes |
| 1.4.1 Use of Color | A | Our UI never uses color alone; patterns and labels accompany color. | Our UI: yes |
| 1.4.3 Contrast (Minimum) | AA | Our UI ≥ 4.5:1; reflow respects user contrast settings. | Our UI: yes |
| 1.4.4 Resize Text | AA | Our UI scales; the overlay does not block app zoom. | Our UI: yes |
| 1.4.8 Visual Presentation | AAA | Reflow: line length, spacing, alignment, font, contrast. | Yes, user-controlled |
| 1.4.10 Reflow | AA | Simplified view reflows to a single column. | Partial |
| 1.4.12 Text Spacing | AA | Reflow enforces spacing without clipping. | Partial |
| 2.1.1 Keyboard | A | The entire overlay is keyboard operable; we do not trap focus in the app. | Our UI: yes |
| 2.1.2 No Keyboard Trap | A | Verified by automated test. | Our UI: yes |
| 2.2.1 Timing Adjustable | A | We never impose timeouts; countdowns are surfaced, not enforced. | Our UI: yes |
| 2.2.2 Pause, Stop, Hide | A | Stop-motion mode pauses/hides animation. | Partial (we hide, not stop the app) |
| 2.3.1 Three Flashes | A | We detect flashing and offer to freeze; our UI never flashes. | Partial |
| 2.4.1 Bypass Blocks | A | The icon grid and simplified view are a bypass. | Yes |
| 2.4.2 Page Titled | A | We use the app/window title in the overlay header. | Yes |
| 2.4.3 Focus Order | A | Our overlay has a logical focus order. | Our UI: yes |
| 2.4.6 Headings and Labels | AA | All our controls have visible labels and headings. | Our UI: yes |
| 2.4.7 Focus Visible | AA | Visible focus ring, ≥ 3:1. | Our UI: yes |
| 2.4.11 Focus Not Obscured | AA | The overlay never covers the focused control without a dismiss affordance. | Our UI: yes |
| 2.5.7 Dragging Movements | AA | We do not introduce dragging; our controls are click/keyboard. | Our UI: yes |
| 2.5.8 Target Size (Minimum) | AA | Our targets ≥ 24 px; large mode ≥ 60 px. | Our UI: yes |
| 3.1.1 Language of Page | A | We detect and declare the language of the overlay. | Our UI: yes |
| 3.1.3 Unusual Words | AAA | Jargon dictionary + definitions. | Yes |
| 3.1.4 Abbreviations | AAA | Acronym expansion. | Yes |
| 3.1.5 Reading Level | AAA | Reading-level targeting + summaries. | Yes |
| 3.2.1 On Focus | A | Our controls do not auto-submit or navigate on focus. | Our UI: yes |
| 3.2.2 On Input | A | Our settings apply on explicit action, with undo. | Our UI: yes |
| 3.2.3 Consistent Navigation | AA | The overlay's navigation is identical across apps. | Yes |
| 3.2.4 Consistent Identification | AA | Same words for the same actions everywhere. | Yes |
| 3.2.6 Consistent Help | A | Help is always in the same place (the "?" tile). | Our UI: yes |
| 3.3.1 Error Identification | A | We surface validation errors in the simplified view. | Partial |
| 3.3.2 Labels or Instructions | A | Our controls have visible labels and inline help. | Our UI: yes |
| 3.3.3 Error Suggestion | AA | We offer "show original" and "help" near errors. | Partial |
| 3.3.7 Redundant Entry | A | One-step mode keeps answers visible; we do not force re-entry. | Partial |
| 3.3.8 Accessible Authentication | AA | We do not add CAPTCHAs or memory-heavy auth. | Our UI: yes |
| 4.1.2 Name, Role, Value | A | All our controls expose name/role/state via the platform API. | Our UI: yes |
| 4.1.3 Status Messages | AA | Our status changes are announced to screen readers. | Our UI: yes |

**Honest summary:** our own UI targets WCAG 2.2 AA. The overlay's effect on the underlying app is an accommodation, not conformance.

---

## 3. COGA mapping

| COGA objective | CurbCut feature |
| --- | --- |
| 1. Understand what things are | Icon labels, role classification, plain labels |
| 2. Find what you need | Icon grid, search, simplified view, headings |
| 3. Clear content | Simplification, chunking, summaries, jargon, numbers |
| 4. Avoid mistakes | Confirmations, undo, visible answers, clear errors |
| 5. Focus | Dimming, stop-motion, interruption inbox, focus sessions |
| 6. No memory reliance | One-step mode, task maps, visible answers, reminders |
| 7. Help and support | Provenance panel, help tile, human-handoff docs |
| 8. Adaptation | Profile graph, per-app layouts, learned simplifications |

---

## 4. Known limitations (published)

1. We cannot see protected/DRM content or secure desktops.
2. We cannot reach into a native app's internals; we cover pixels, not code.
3. Detection is probabilistic; we publish mAP and CER and a per-app confidence indicator.
4. Simplification can be wrong; we check meaning and always show the original.
5. We do not support every language equally; we publish the language matrix.
6. Linux Wayland support depends on the portal; some distros are beta.
7. We are not a screen reader and do not replace one.
8. We are not a compliance tool and do not make apps conform.

---

## 5. VPAT

The VPAT generator (`curbcut-cli vpat`) uses this table plus the automated audit results to produce a draft. A human reviews it before publication. We never auto-publish a "conformant" row for any third-party app.
