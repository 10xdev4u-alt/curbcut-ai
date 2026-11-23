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

| Criterion | Level | CurbCut behavior | Status | Test |
| --- | --- | --- | --- | --- |
| 1.1.1 Non-text Content | A | Icon captioning + a11y names provide text alternatives for icons we detect. | Partially supports; only for detected icons | [automated: `crates/curbcut-understand/src/icon_semantics/tests.rs#a_confident_caption_is_used`](../crates/curbcut-understand/src/icon_semantics/tests.rs#a_confident_caption_is_used) |
| 1.3.1 Info and Relationships | A | The simplified view preserves headings, lists, and labels; we never flatten structure. | Partially supports; depends on detection | [automated: `crates/curbcut-understand/src/chunking.rs#a_heading_starts_a_new_chunk`](../crates/curbcut-understand/src/chunking.rs#a_heading_starts_a_new_chunk) |
| 1.3.2 Meaningful Sequence | A | Reading-order analysis drives the simplified view. | Partially supports | [automated: `crates/curbcut-a11y/tests/differential.rs#the_fixture_set_matches_the_published_baseline`](../crates/curbcut-a11y/tests/differential.rs#the_fixture_set_matches_the_published_baseline) |
| 1.3.3 Sensory Characteristics | A | We never rely on shape/color alone; labels are always textual. | Supports | [manual: `docs/A11Y_AUDIT.md#133-sensory-characteristics`](A11Y_AUDIT.md#133-sensory-characteristics) |
| 1.4.1 Use of Color | A | Our UI never uses color alone; patterns and labels accompany color. | Supports | [manual: `docs/A11Y_AUDIT.md#141-use-of-color`](A11Y_AUDIT.md#141-use-of-color) |
| 1.4.3 Contrast (Minimum) | AA | Our UI ≥ 4.5:1; reflow respects user contrast settings. | Supports | [automated: `crates/curbcut-adapt/src/typography/tests.rs#a_palette_that_fails_aa_is_reported`](../crates/curbcut-adapt/src/typography/tests.rs#a_palette_that_fails_aa_is_reported) |
| 1.4.4 Resize Text | AA | Our UI scales; the overlay does not block app zoom. | Supports | [manual: `docs/A11Y_AUDIT.md#144-resize-text`](A11Y_AUDIT.md#144-resize-text) |
| 1.4.8 Visual Presentation | AAA | Reflow: line length, spacing, alignment, font, contrast. | Supports; user-controlled | [automated: `crates/curbcut-adapt/src/typography/tests.rs#a_styled_reflow_preserves_words_and_never_clips`](../crates/curbcut-adapt/src/typography/tests.rs#a_styled_reflow_preserves_words_and_never_clips) |
| 1.4.10 Reflow | AA | Simplified view reflows to a single column. | Partially supports | [automated: `crates/curbcut-adapt/src/layout/tests.rs#reflow_breaks_lines_without_losing_words`](../crates/curbcut-adapt/src/layout/tests.rs#reflow_breaks_lines_without_losing_words) |
| 1.4.12 Text Spacing | AA | Reflow enforces spacing without clipping. | Partially supports | [automated: `crates/curbcut-adapt/src/typography/tests.rs#a_short_line_height_is_raised_to_the_font_size`](../crates/curbcut-adapt/src/typography/tests.rs#a_short_line_height_is_raised_to_the_font_size) |
| 2.1.1 Keyboard | A | The entire overlay is keyboard operable; we do not trap focus in the app. | Supports | [automated: `crates/curbcut-input/src/focus_order/tests.rs#a_new_order_focuses_the_first_enabled_item`](../crates/curbcut-input/src/focus_order/tests.rs#a_new_order_focuses_the_first_enabled_item) |
| 2.1.2 No Keyboard Trap | A | Tab and Escape always move focus; the focus order validator rejects a trapping item. | Supports | [automated: `crates/curbcut-input/src/focus_order/tests.rs#validate_reports_a_single_trap`](../crates/curbcut-input/src/focus_order/tests.rs#validate_reports_a_single_trap) |
| 2.2.1 Timing Adjustable | A | We never impose timeouts; countdowns are surfaced, not enforced. | Supports | [automated: `crates/curbcut-vision/src/motion_report/tests.rs#a_small_change_in_a_small_region_is_a_countdown`](../crates/curbcut-vision/src/motion_report/tests.rs#a_small_change_in_a_small_region_is_a_countdown) |
| 2.2.2 Pause, Stop, Hide | A | Stop-motion mode pauses/hides animation. | Partially supports; we hide, not stop the app | [automated: `crates/curbcut-render/src/animation/tests.rs#reduced_motion_refuses_non_essential_motion`](../crates/curbcut-render/src/animation/tests.rs#reduced_motion_refuses_non_essential_motion) |
| 2.3.1 Three Flashes | A | We detect flashing and offer to freeze; our UI never flashes. | Partially supports | [automated: `crates/curbcut-render/src/animation/tests.rs#vestibular_triggers_are_always_refused`](../crates/curbcut-render/src/animation/tests.rs#vestibular_triggers_are_always_refused) |
| 2.4.1 Bypass Blocks | A | The icon grid and simplified view are a bypass. | Supports | [manual: `docs/A11Y_AUDIT.md#241-bypass-blocks`](A11Y_AUDIT.md#241-bypass-blocks) |
| 2.4.2 Page Titled | A | We use the app/window title in the overlay header. | Supports | [manual: `docs/A11Y_AUDIT.md#242-page-titled`](A11Y_AUDIT.md#242-page-titled) |
| 2.4.3 Focus Order | A | Our overlay has a logical focus order. | Supports | [automated: `crates/curbcut-input/src/focus_order/tests.rs#advance_and_back_wrap_and_skip_disabled`](../crates/curbcut-input/src/focus_order/tests.rs#advance_and_back_wrap_and_skip_disabled) |
| 2.4.6 Headings and Labels | AA | All our controls have visible labels and headings. | Supports | [manual: `docs/A11Y_AUDIT.md#246-headings-and-labels`](A11Y_AUDIT.md#246-headings-and-labels) |
| 2.4.7 Focus Visible | AA | Visible focus ring, ≥ 3:1. | Supports | [manual: `docs/A11Y_AUDIT.md#247-focus-visible`](A11Y_AUDIT.md#247-focus-visible) |
| 2.4.11 Focus Not Obscured | AA | The overlay never covers the focused control without a dismiss affordance. | Supports | [automated: `crates/curbcut-adapt/src/dim/tests.rs#the_focused_control_is_left_undimmed`](../crates/curbcut-adapt/src/dim/tests.rs#the_focused_control_is_left_undimmed) |
| 2.5.7 Dragging Movements | AA | We do not introduce dragging; our controls are click/keyboard. | Supports | [manual: `docs/A11Y_AUDIT.md#257-dragging-movements`](A11Y_AUDIT.md#257-dragging-movements) |
| 2.5.8 Target Size (Minimum) | AA | Our targets ≥ 24 px; large mode ≥ 60 px. | Supports | [automated: `crates/curbcut-render/src/targets/tests.rs#a_small_target_is_scaled_up_to_the_minimum`](../crates/curbcut-render/src/targets/tests.rs#a_small_target_is_scaled_up_to_the_minimum) |
| 3.1.1 Language of Page | A | We detect and declare the language of the overlay. | Supports | [automated: `crates/curbcut-understand/src/jargon/tests.rs#locales_are_normalised`](../crates/curbcut-understand/src/jargon/tests.rs#locales_are_normalised) |
| 3.1.3 Unusual Words | AAA | Jargon dictionary + definitions. | Supports | [automated: `crates/curbcut-understand/src/jargon/tests.rs#a_user_entry_overrides_a_builtin`](../crates/curbcut-understand/src/jargon/tests.rs#a_user_entry_overrides_a_builtin) |
| 3.1.4 Abbreviations | AAA | Acronym expansion. | Supports | [automated: `crates/curbcut-understand/src/jargon/tests.rs#known_acronyms_are_expanded_with_offsets`](../crates/curbcut-understand/src/jargon/tests.rs#known_acronyms_are_expanded_with_offsets) |
| 3.1.5 Reading Level | AAA | Reading-level targeting + summaries. | Supports | [automated: `crates/curbcut-understand/src/reading.rs#a_target_is_met_when_the_text_is_at_or_below_it`](../crates/curbcut-understand/src/reading.rs#a_target_is_met_when_the_text_is_at_or_below_it) |
| 3.2.1 On Focus | A | Our controls do not auto-submit or navigate on focus. | Supports | [manual: `docs/A11Y_AUDIT.md#321-on-focus`](A11Y_AUDIT.md#321-on-focus) |
| 3.2.2 On Input | A | Our settings apply on explicit action, with undo. | Supports | [manual: `docs/A11Y_AUDIT.md#322-on-input`](A11Y_AUDIT.md#322-on-input) |
| 3.2.3 Consistent Navigation | AA | The overlay's navigation is identical across apps. | Supports | [manual: `docs/A11Y_AUDIT.md#323-consistent-navigation`](A11Y_AUDIT.md#323-consistent-navigation) |
| 3.2.4 Consistent Identification | AA | Same words for the same actions everywhere. | Supports | [manual: `docs/A11Y_AUDIT.md#324-consistent-identification`](A11Y_AUDIT.md#324-consistent-identification) |
| 3.2.6 Consistent Help | A | Help is always in the same place (the "?" tile). | Supports | [manual: `docs/A11Y_AUDIT.md#326-consistent-help`](A11Y_AUDIT.md#326-consistent-help) |
| 3.3.1 Error Identification | A | We surface validation errors in the simplified view. | Partially supports | [manual: `docs/A11Y_AUDIT.md#331-error-identification`](A11Y_AUDIT.md#331-error-identification) |
| 3.3.2 Labels or Instructions | A | Our controls have visible labels and inline help. | Supports | [manual: `docs/A11Y_AUDIT.md#332-labels-or-instructions`](A11Y_AUDIT.md#332-labels-or-instructions) |
| 3.3.3 Error Suggestion | AA | We offer "show original" and "help" near errors. | Partially supports | [manual: `docs/A11Y_AUDIT.md#333-error-suggestion`](A11Y_AUDIT.md#333-error-suggestion) |
| 3.3.7 Redundant Entry | A | One-step mode keeps answers visible; we do not force re-entry. | Partially supports | [manual: `docs/A11Y_AUDIT.md#337-redundant-entry`](A11Y_AUDIT.md#337-redundant-entry) |
| 3.3.8 Accessible Authentication | AA | We do not add CAPTCHAs or memory-heavy auth. | Supports | [manual: `docs/A11Y_AUDIT.md#338-accessible-authentication`](A11Y_AUDIT.md#338-accessible-authentication) |
| 4.1.2 Name, Role, Value | A | All our controls expose name/role/state via the platform API. | Supports | [automated: `crates/curbcut-a11y/src/mapping.rs#every_table_row_maps_to_a_concrete_role`](../crates/curbcut-a11y/src/mapping.rs#every_table_row_maps_to_a_concrete_role) |
| 4.1.3 Status Messages | AA | Our status changes are announced to screen readers. | Supports | [manual: `docs/A11Y_AUDIT.md#413-status-messages`](A11Y_AUDIT.md#413-status-messages) |

**Honest summary:** our own UI targets WCAG 2.2 AA. The overlay's effect on the underlying app is an accommodation, not conformance.

Every row names the test that backs it. A row marked `manual` is backed by a
procedure in [`A11Y_AUDIT.md`](A11Y_AUDIT.md), not by an automated test; a row
marked `automated` names a test that runs in CI.

---

## 3. COGA mapping

| COGA objective | CurbCut feature | Test |
| --- | --- | --- |
| Understand what things are | Icon labels, role classification, plain labels | [automated: `crates/curbcut-understand/src/icon_semantics/tests.rs#an_accessible_label_wins`](../crates/curbcut-understand/src/icon_semantics/tests.rs#an_accessible_label_wins) |
| Find what you need | Icon grid, search, simplified view, headings | [automated: `crates/curbcut-understand/src/chunking.rs#a_heading_starts_a_new_chunk`](../crates/curbcut-understand/src/chunking.rs#a_heading_starts_a_new_chunk) |
| Clear content | Simplification, chunking, summaries, jargon, numbers | [automated: `crates/curbcut-understand/src/summarize/tests.rs#selection_is_by_score_but_output_is_in_reading_order`](../crates/curbcut-understand/src/summarize/tests.rs#selection_is_by_score_but_output_is_in_reading_order) |
| Avoid mistakes | Confirmations, undo, visible answers, clear errors | [automated: `crates/curbcut-understand/src/actions/tests.rs#a_safe_request_needs_no_confirmation`](../crates/curbcut-understand/src/actions/tests.rs#a_safe_request_needs_no_confirmation) |
| Focus | Dimming, stop-motion, interruption inbox, focus sessions | [automated: `crates/curbcut-adapt/src/dim/tests.rs#the_focused_control_is_left_undimmed`](../crates/curbcut-adapt/src/dim/tests.rs#the_focused_control_is_left_undimmed) |
| No memory reliance | One-step mode, task maps, visible answers, reminders | [automated: `crates/curbcut-understand/src/actions/tests.rs#every_listed_request_gets_a_preview`](../crates/curbcut-understand/src/actions/tests.rs#every_listed_request_gets_a_preview) |
| Help and support | Provenance panel, help tile, human-handoff docs | [automated: `crates/curbcut-adapt/src/provenance/tests.rs#a_change_links_to_its_rule`](../crates/curbcut-adapt/src/provenance/tests.rs#a_change_links_to_its_rule) |
| Adaptation | Profile graph, per-app layouts, learned simplifications | [automated: `crates/curbcut-adapt/src/profile/tests.rs#every_built_in_kind_maps_to_its_id`](../crates/curbcut-adapt/src/profile/tests.rs#every_built_in_kind_maps_to_its_id) |

---

## 4. Known limitations (published)

The limitations page is generated from the same data. See
[`LIMITATIONS.md`](LIMITATIONS.md).

---

## 5. VPAT

The VPAT generator (`curbcut-cli vpat`) uses this table plus the automated
audit results to produce a draft. A human reviews it before publication. We
never auto-publish a "conformant" row for any third-party app. See
[`VPAT.md`](VPAT.md).
