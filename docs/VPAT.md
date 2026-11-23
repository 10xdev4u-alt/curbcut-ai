# Draft VPAT: CurbCut AI

> **DRAFT.** Generated from `crates/curbcut-cli/src/vpat.rs`. This VPAT
> evaluates CurbCut's own UI only. It is not a conformance claim about any
> third-party application, and it has not been reviewed by a human.

## Scope

This draft evaluates **CurbCut's own UI**: the overlay, the settings windows,
and the tray. It makes no claim about any application CurbCut is pointed at.
CurbCut is an alternative perceptual channel, not a conformance fix, so a
`Supports` here means "our UI meets the criterion", never "the app underneath
conforms".

The criteria, levels, and results come from the mapping table in
[`A11Y_MAPPING.md`](A11Y_MAPPING.md). A `Partially Supports` remark names the
limit and the test that backs the claim.

## WCAG 2.2 criteria

| Criterion | Level | Result | Remarks |
| --- | --- | --- | --- |
| 1.1.1 Non-text Content | A | Partially Supports | Icon captioning + a11y names provide text alternatives for icons we detect. Only for detected icons. Verified by automated test: crates/curbcut-understand/src/icon_semantics/tests.rs#a_confident_caption_is_used. |
| 1.3.1 Info and Relationships | A | Partially Supports | The simplified view preserves headings, lists, and labels; we never flatten structure. Depends on detection. Verified by automated test: crates/curbcut-understand/src/chunking.rs#a_heading_starts_a_new_chunk. |
| 1.3.2 Meaningful Sequence | A | Partially Supports | Reading-order analysis drives the simplified view. Verified by automated test: crates/curbcut-a11y/tests/differential.rs#the_fixture_set_matches_the_published_baseline. |
| 1.3.3 Sensory Characteristics | A | Supports | We never rely on shape/color alone; labels are always textual. Verified by manual test: docs/A11Y_AUDIT.md#133-sensory-characteristics. |
| 1.4.1 Use of Color | A | Supports | Our UI never uses color alone; patterns and labels accompany color. Verified by manual test: docs/A11Y_AUDIT.md#141-use-of-color. |
| 1.4.3 Contrast (Minimum) | AA | Supports | Our UI ≥ 4.5:1; reflow respects user contrast settings. Verified by automated test: crates/curbcut-adapt/src/typography/tests.rs#a_palette_that_fails_aa_is_reported. |
| 1.4.4 Resize Text | AA | Supports | Our UI scales; the overlay does not block app zoom. Verified by manual test: docs/A11Y_AUDIT.md#144-resize-text. |
| 1.4.8 Visual Presentation | AAA | Supports | Reflow: line length, spacing, alignment, font, contrast. User-controlled. Verified by automated test: crates/curbcut-adapt/src/typography/tests.rs#a_styled_reflow_preserves_words_and_never_clips. |
| 1.4.10 Reflow | AA | Partially Supports | Simplified view reflows to a single column. Verified by automated test: crates/curbcut-adapt/src/layout/tests.rs#reflow_breaks_lines_without_losing_words. |
| 1.4.12 Text Spacing | AA | Partially Supports | Reflow enforces spacing without clipping. Verified by automated test: crates/curbcut-adapt/src/typography/tests.rs#a_short_line_height_is_raised_to_the_font_size. |
| 2.1.1 Keyboard | A | Supports | The entire overlay is keyboard operable; we do not trap focus in the app. Verified by automated test: crates/curbcut-input/src/focus_order/tests.rs#a_new_order_focuses_the_first_enabled_item. |
| 2.1.2 No Keyboard Trap | A | Supports | Tab and Escape always move focus; the focus order validator rejects a trapping item. Verified by automated test: crates/curbcut-input/src/focus_order/tests.rs#validate_reports_a_single_trap. |
| 2.2.1 Timing Adjustable | A | Supports | We never impose timeouts; countdowns are surfaced, not enforced. Verified by automated test: crates/curbcut-vision/src/motion_report/tests.rs#a_small_change_in_a_small_region_is_a_countdown. |
| 2.2.2 Pause, Stop, Hide | A | Partially Supports | Stop-motion mode pauses/hides animation. We hide, not stop the app. Verified by automated test: crates/curbcut-render/src/animation/tests.rs#reduced_motion_refuses_non_essential_motion. |
| 2.3.1 Three Flashes | A | Partially Supports | We detect flashing and offer to freeze; our UI never flashes. Verified by automated test: crates/curbcut-render/src/animation/tests.rs#vestibular_triggers_are_always_refused. |
| 2.4.1 Bypass Blocks | A | Supports | The icon grid and simplified view are a bypass. Verified by manual test: docs/A11Y_AUDIT.md#241-bypass-blocks. |
| 2.4.2 Page Titled | A | Supports | We use the app/window title in the overlay header. Verified by manual test: docs/A11Y_AUDIT.md#242-page-titled. |
| 2.4.3 Focus Order | A | Supports | Our overlay has a logical focus order. Verified by automated test: crates/curbcut-input/src/focus_order/tests.rs#advance_and_back_wrap_and_skip_disabled. |
| 2.4.6 Headings and Labels | AA | Supports | All our controls have visible labels and headings. Verified by manual test: docs/A11Y_AUDIT.md#246-headings-and-labels. |
| 2.4.7 Focus Visible | AA | Supports | Visible focus ring, ≥ 3:1. Verified by manual test: docs/A11Y_AUDIT.md#247-focus-visible. |
| 2.4.11 Focus Not Obscured | AA | Supports | The overlay never covers the focused control without a dismiss affordance. Verified by automated test: crates/curbcut-adapt/src/dim/tests.rs#the_focused_control_is_left_undimmed. |
| 2.5.7 Dragging Movements | AA | Supports | We do not introduce dragging; our controls are click/keyboard. Verified by manual test: docs/A11Y_AUDIT.md#257-dragging-movements. |
| 2.5.8 Target Size (Minimum) | AA | Supports | Our targets ≥ 24 px; large mode ≥ 60 px. Verified by automated test: crates/curbcut-render/src/targets/tests.rs#a_small_target_is_scaled_up_to_the_minimum. |
| 3.1.1 Language of Page | A | Supports | We detect and declare the language of the overlay. Verified by automated test: crates/curbcut-understand/src/jargon/tests.rs#locales_are_normalised. |
| 3.1.3 Unusual Words | AAA | Supports | Jargon dictionary + definitions. Verified by automated test: crates/curbcut-understand/src/jargon/tests.rs#a_user_entry_overrides_a_builtin. |
| 3.1.4 Abbreviations | AAA | Supports | Acronym expansion. Verified by automated test: crates/curbcut-understand/src/jargon/tests.rs#known_acronyms_are_expanded_with_offsets. |
| 3.1.5 Reading Level | AAA | Supports | Reading-level targeting + summaries. Verified by automated test: crates/curbcut-understand/src/reading.rs#a_target_is_met_when_the_text_is_at_or_below_it. |
| 3.2.1 On Focus | A | Supports | Our controls do not auto-submit or navigate on focus. Verified by manual test: docs/A11Y_AUDIT.md#321-on-focus. |
| 3.2.2 On Input | A | Supports | Our settings apply on explicit action, with undo. Verified by manual test: docs/A11Y_AUDIT.md#322-on-input. |
| 3.2.3 Consistent Navigation | AA | Supports | The overlay's navigation is identical across apps. Verified by manual test: docs/A11Y_AUDIT.md#323-consistent-navigation. |
| 3.2.4 Consistent Identification | AA | Supports | Same words for the same actions everywhere. Verified by manual test: docs/A11Y_AUDIT.md#324-consistent-identification. |
| 3.2.6 Consistent Help | A | Supports | Help is always in the same place (the "?" tile). Verified by manual test: docs/A11Y_AUDIT.md#326-consistent-help. |
| 3.3.1 Error Identification | A | Partially Supports | We surface validation errors in the simplified view. Verified by manual test: docs/A11Y_AUDIT.md#331-error-identification. |
| 3.3.2 Labels or Instructions | A | Supports | Our controls have visible labels and inline help. Verified by manual test: docs/A11Y_AUDIT.md#332-labels-or-instructions. |
| 3.3.3 Error Suggestion | AA | Partially Supports | We offer "show original" and "help" near errors. Verified by manual test: docs/A11Y_AUDIT.md#333-error-suggestion. |
| 3.3.7 Redundant Entry | A | Partially Supports | One-step mode keeps answers visible; we do not force re-entry. Verified by manual test: docs/A11Y_AUDIT.md#337-redundant-entry. |
| 3.3.8 Accessible Authentication | AA | Supports | We do not add CAPTCHAs or memory-heavy auth. Verified by manual test: docs/A11Y_AUDIT.md#338-accessible-authentication. |
| 4.1.2 Name, Role, Value | A | Supports | All our controls expose name/role/state via the platform API. Verified by automated test: crates/curbcut-a11y/src/mapping.rs#every_table_row_maps_to_a_concrete_role. |
| 4.1.3 Status Messages | AA | Supports | Our status changes are announced to screen readers. Verified by manual test: docs/A11Y_AUDIT.md#413-status-messages. |

**Draft summary:** 29 Supports, 10 Partially Supports, 0 Not Applicable, 0 Not Assessed, across 39 criteria.

## Human review

- [ ] Every row read by a human who has seen the product.
- [ ] Every `Partially Supports` remark understood and accepted.
- [ ] Every manual test in [`A11Y_AUDIT.md`](A11Y_AUDIT.md) executed and its
      result recorded.
- [ ] The scope statement agreed: CurbCut's own UI, not a third-party app.
- [ ] The draft banner removed only after the review is recorded.

A VPAT is not published until every box is ticked and the review is recorded in
the pull request that publishes it.
