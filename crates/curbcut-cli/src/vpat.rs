//! The WCAG 2.2 / COGA mapping, the draft VPAT, and the limitations page.
//!
//! This module is the single source of truth for the public accessibility
//! claims. [`WCAG`], [`COGA`], and [`LIMITATIONS`] are data, and the `render_*`
//! functions turn them into the published documents. A row is only publishable
//! when it names a test that backs it, so [`MappingRow::test`] and
//! [`CogaRow::test`] are part of the claim, not a footnote.
//!
//! The VPAT is a **draft** for CurbCut's own UI. It is never a conformance
//! claim about a third-party application, and it is never published without a
//! human review. See [`../../../docs/VPAT.md`](../../../docs/VPAT.md) and the
//! manual procedures in
//! [`../../../docs/A11Y_AUDIT.md`](../../../docs/A11Y_AUDIT.md).

use std::fmt;

/// A WCAG conformance level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    /// Level A.
    A,
    /// Level AA.
    Aa,
    /// Level AAA.
    Aaa,
}

impl Level {
    /// The short label used in the tables.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::Aa => "AA",
            Self::Aaa => "AAA",
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The status of a mapping row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// CurbCut's own UI meets the criterion.
    Supports,
    /// The overlay helps, but the criterion is not fully met.
    Partial,
    /// The criterion does not apply to CurbCut's own UI.
    NotApplicable,
    /// The criterion has not been assessed.
    NotAssessed,
}

impl Status {
    /// The label used in the mapping table.
    pub fn label(self) -> &'static str {
        match self {
            Self::Supports => "Supports",
            Self::Partial => "Partially supports",
            Self::NotApplicable => "Not applicable",
            Self::NotAssessed => "Not assessed",
        }
    }

    /// The VPAT 2.5 result term.
    pub fn vpat_result(self) -> &'static str {
        match self {
            Self::Supports => "Supports",
            Self::Partial => "Partially Supports",
            Self::NotApplicable => "Not Applicable",
            Self::NotAssessed => "Not Assessed",
        }
    }
}

/// How a row is verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestKind {
    /// An automated test in the repository.
    Automated,
    /// A manual procedure in `docs/A11Y_AUDIT.md`.
    Manual,
}

impl TestKind {
    /// The label used in the tables.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Automated => "automated",
            Self::Manual => "manual",
        }
    }
}

/// One WCAG 2.2 criterion row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingRow {
    /// Criterion id and name, such as `1.1.1 Non-text Content`.
    pub criterion: &'static str,
    /// Conformance level.
    pub level: Level,
    /// What CurbCut does about it.
    pub behavior: &'static str,
    /// Status of the criterion for CurbCut's own UI.
    pub status: Status,
    /// A short qualifier, empty when the status speaks for itself.
    pub note: &'static str,
    /// Repository-relative test target, optionally with a `#fragment`.
    pub test: &'static str,
    /// Whether the target is automated or manual.
    pub kind: TestKind,
}

impl MappingRow {
    /// The numeric criterion id, such as `1.1.1`.
    pub fn id(&self) -> &'static str {
        match self.criterion.split_once(' ') {
            Some((id, _)) => id,
            None => self.criterion,
        }
    }

    /// The status cell: the label, plus the qualifier after a semicolon.
    pub fn status_cell(&self) -> String {
        if self.note.is_empty() {
            self.status.label().to_string()
        } else {
            format!("{}; {}", self.status.label(), self.note)
        }
    }

    /// The remark cell for the VPAT: the behavior, the qualifier as a sentence,
    /// and the test that backs it.
    pub fn remark(&self) -> String {
        let mut text = self.behavior.trim_end_matches('.').to_string();
        if !self.note.is_empty() {
            text.push_str(". ");
            text.push_str(&capitalize(self.note));
        }
        if !text.ends_with('.') {
            text.push('.');
        }
        text.push_str(&format!(
            " Verified by {} test: {}.",
            self.kind.as_str(),
            self.test
        ));
        text
    }
}

/// One W3C COGA objective row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CogaRow {
    /// The objective, such as `Understand what things are`.
    pub objective: &'static str,
    /// The CurbCut features that address it.
    pub feature: &'static str,
    /// Repository-relative test target, optionally with a `#fragment`.
    pub test: &'static str,
    /// Whether the target is automated or manual.
    pub kind: TestKind,
}

/// The WCAG 2.2 criteria we make a claim about.
///
/// The order is the criterion order. Every row names a test; a row without a
/// real test must not be added.
pub const WCAG: &[MappingRow] = &[
    MappingRow {
        criterion: "1.1.1 Non-text Content",
        level: Level::A,
        behavior: "Icon captioning + a11y names provide text alternatives for icons we detect.",
        status: Status::Partial,
        note: "only for detected icons",
        test: "crates/curbcut-understand/src/icon_semantics/tests.rs#a_confident_caption_is_used",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "1.3.1 Info and Relationships",
        level: Level::A,
        behavior: "The simplified view preserves headings, lists, and labels; we never flatten structure.",
        status: Status::Partial,
        note: "depends on detection",
        test: "crates/curbcut-understand/src/chunking.rs#a_heading_starts_a_new_chunk",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "1.3.2 Meaningful Sequence",
        level: Level::A,
        behavior: "Reading-order analysis drives the simplified view.",
        status: Status::Partial,
        note: "",
        test: "crates/curbcut-a11y/tests/differential.rs#the_fixture_set_matches_the_published_baseline",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "1.3.3 Sensory Characteristics",
        level: Level::A,
        behavior: "We never rely on shape/color alone; labels are always textual.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#133-sensory-characteristics",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "1.4.1 Use of Color",
        level: Level::A,
        behavior: "Our UI never uses color alone; patterns and labels accompany color.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#141-use-of-color",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "1.4.3 Contrast (Minimum)",
        level: Level::Aa,
        behavior: "Our UI ≥ 4.5:1; reflow respects user contrast settings.",
        status: Status::Supports,
        note: "",
        test: "crates/curbcut-adapt/src/typography/tests.rs#a_palette_that_fails_aa_is_reported",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "1.4.4 Resize Text",
        level: Level::Aa,
        behavior: "Our UI scales; the overlay does not block app zoom.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#144-resize-text",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "1.4.8 Visual Presentation",
        level: Level::Aaa,
        behavior: "Reflow: line length, spacing, alignment, font, contrast.",
        status: Status::Supports,
        note: "user-controlled",
        test: "crates/curbcut-adapt/src/typography/tests.rs#a_styled_reflow_preserves_words_and_never_clips",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "1.4.10 Reflow",
        level: Level::Aa,
        behavior: "Simplified view reflows to a single column.",
        status: Status::Partial,
        note: "",
        test: "crates/curbcut-adapt/src/layout/tests.rs#reflow_breaks_lines_without_losing_words",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "1.4.12 Text Spacing",
        level: Level::Aa,
        behavior: "Reflow enforces spacing without clipping.",
        status: Status::Partial,
        note: "",
        test: "crates/curbcut-adapt/src/typography/tests.rs#a_short_line_height_is_raised_to_the_font_size",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "2.1.1 Keyboard",
        level: Level::A,
        behavior: "The entire overlay is keyboard operable; we do not trap focus in the app.",
        status: Status::Supports,
        note: "",
        test: "crates/curbcut-input/src/focus_order/tests.rs#a_new_order_focuses_the_first_enabled_item",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "2.1.2 No Keyboard Trap",
        level: Level::A,
        behavior: "Tab and Escape always move focus; the focus order validator rejects a trapping item.",
        status: Status::Supports,
        note: "",
        test: "crates/curbcut-input/src/focus_order/tests.rs#validate_reports_a_single_trap",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "2.2.1 Timing Adjustable",
        level: Level::A,
        behavior: "We never impose timeouts; countdowns are surfaced, not enforced.",
        status: Status::Supports,
        note: "",
        test: "crates/curbcut-vision/src/motion_report/tests.rs#a_small_change_in_a_small_region_is_a_countdown",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "2.2.2 Pause, Stop, Hide",
        level: Level::A,
        behavior: "Stop-motion mode pauses/hides animation.",
        status: Status::Partial,
        note: "we hide, not stop the app",
        test: "crates/curbcut-render/src/animation/tests.rs#reduced_motion_refuses_non_essential_motion",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "2.3.1 Three Flashes",
        level: Level::A,
        behavior: "We detect flashing and offer to freeze; our UI never flashes.",
        status: Status::Partial,
        note: "",
        test: "crates/curbcut-render/src/animation/tests.rs#vestibular_triggers_are_always_refused",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "2.4.1 Bypass Blocks",
        level: Level::A,
        behavior: "The icon grid and simplified view are a bypass.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#241-bypass-blocks",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "2.4.2 Page Titled",
        level: Level::A,
        behavior: "We use the app/window title in the overlay header.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#242-page-titled",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "2.4.3 Focus Order",
        level: Level::A,
        behavior: "Our overlay has a logical focus order.",
        status: Status::Supports,
        note: "",
        test: "crates/curbcut-input/src/focus_order/tests.rs#advance_and_back_wrap_and_skip_disabled",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "2.4.6 Headings and Labels",
        level: Level::Aa,
        behavior: "All our controls have visible labels and headings.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#246-headings-and-labels",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "2.4.7 Focus Visible",
        level: Level::Aa,
        behavior: "Visible focus ring, ≥ 3:1.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#247-focus-visible",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "2.4.11 Focus Not Obscured",
        level: Level::Aa,
        behavior: "The overlay never covers the focused control without a dismiss affordance.",
        status: Status::Supports,
        note: "",
        test: "crates/curbcut-adapt/src/dim/tests.rs#the_focused_control_is_left_undimmed",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "2.5.7 Dragging Movements",
        level: Level::Aa,
        behavior: "We do not introduce dragging; our controls are click/keyboard.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#257-dragging-movements",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "2.5.8 Target Size (Minimum)",
        level: Level::Aa,
        behavior: "Our targets ≥ 24 px; large mode ≥ 60 px.",
        status: Status::Supports,
        note: "",
        test: "crates/curbcut-render/src/targets/tests.rs#a_small_target_is_scaled_up_to_the_minimum",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "3.1.1 Language of Page",
        level: Level::A,
        behavior: "We detect and declare the language of the overlay.",
        status: Status::Supports,
        note: "",
        test: "crates/curbcut-understand/src/jargon/tests.rs#locales_are_normalised",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "3.1.3 Unusual Words",
        level: Level::Aaa,
        behavior: "Jargon dictionary + definitions.",
        status: Status::Supports,
        note: "",
        test: "crates/curbcut-understand/src/jargon/tests.rs#a_user_entry_overrides_a_builtin",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "3.1.4 Abbreviations",
        level: Level::Aaa,
        behavior: "Acronym expansion.",
        status: Status::Supports,
        note: "",
        test: "crates/curbcut-understand/src/jargon/tests.rs#known_acronyms_are_expanded_with_offsets",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "3.1.5 Reading Level",
        level: Level::Aaa,
        behavior: "Reading-level targeting + summaries.",
        status: Status::Supports,
        note: "",
        test: "crates/curbcut-understand/src/reading.rs#a_target_is_met_when_the_text_is_at_or_below_it",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "3.2.1 On Focus",
        level: Level::A,
        behavior: "Our controls do not auto-submit or navigate on focus.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#321-on-focus",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "3.2.2 On Input",
        level: Level::A,
        behavior: "Our settings apply on explicit action, with undo.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#322-on-input",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "3.2.3 Consistent Navigation",
        level: Level::Aa,
        behavior: "The overlay's navigation is identical across apps.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#323-consistent-navigation",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "3.2.4 Consistent Identification",
        level: Level::Aa,
        behavior: "Same words for the same actions everywhere.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#324-consistent-identification",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "3.2.6 Consistent Help",
        level: Level::A,
        behavior: "Help is always in the same place (the \"?\" tile).",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#326-consistent-help",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "3.3.1 Error Identification",
        level: Level::A,
        behavior: "We surface validation errors in the simplified view.",
        status: Status::Partial,
        note: "",
        test: "docs/A11Y_AUDIT.md#331-error-identification",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "3.3.2 Labels or Instructions",
        level: Level::A,
        behavior: "Our controls have visible labels and inline help.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#332-labels-or-instructions",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "3.3.3 Error Suggestion",
        level: Level::Aa,
        behavior: "We offer \"show original\" and \"help\" near errors.",
        status: Status::Partial,
        note: "",
        test: "docs/A11Y_AUDIT.md#333-error-suggestion",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "3.3.7 Redundant Entry",
        level: Level::A,
        behavior: "One-step mode keeps answers visible; we do not force re-entry.",
        status: Status::Partial,
        note: "",
        test: "docs/A11Y_AUDIT.md#337-redundant-entry",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "3.3.8 Accessible Authentication",
        level: Level::Aa,
        behavior: "We do not add CAPTCHAs or memory-heavy auth.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#338-accessible-authentication",
        kind: TestKind::Manual,
    },
    MappingRow {
        criterion: "4.1.2 Name, Role, Value",
        level: Level::A,
        behavior: "All our controls expose name/role/state via the platform API.",
        status: Status::Supports,
        note: "",
        test: "crates/curbcut-a11y/src/mapping.rs#every_table_row_maps_to_a_concrete_role",
        kind: TestKind::Automated,
    },
    MappingRow {
        criterion: "4.1.3 Status Messages",
        level: Level::Aa,
        behavior: "Our status changes are announced to screen readers.",
        status: Status::Supports,
        note: "",
        test: "docs/A11Y_AUDIT.md#413-status-messages",
        kind: TestKind::Manual,
    },
];

/// The W3C COGA objectives we make a claim about.
pub const COGA: &[CogaRow] = &[
    CogaRow {
        objective: "Understand what things are",
        feature: "Icon labels, role classification, plain labels",
        test: "crates/curbcut-understand/src/icon_semantics/tests.rs#an_accessible_label_wins",
        kind: TestKind::Automated,
    },
    CogaRow {
        objective: "Find what you need",
        feature: "Icon grid, search, simplified view, headings",
        test: "crates/curbcut-understand/src/chunking.rs#a_heading_starts_a_new_chunk",
        kind: TestKind::Automated,
    },
    CogaRow {
        objective: "Clear content",
        feature: "Simplification, chunking, summaries, jargon, numbers",
        test: "crates/curbcut-understand/src/summarize/tests.rs#selection_is_by_score_but_output_is_in_reading_order",
        kind: TestKind::Automated,
    },
    CogaRow {
        objective: "Avoid mistakes",
        feature: "Confirmations, undo, visible answers, clear errors",
        test: "crates/curbcut-understand/src/actions/tests.rs#a_safe_request_needs_no_confirmation",
        kind: TestKind::Automated,
    },
    CogaRow {
        objective: "Focus",
        feature: "Dimming, stop-motion, interruption inbox, focus sessions",
        test: "crates/curbcut-adapt/src/dim/tests.rs#the_focused_control_is_left_undimmed",
        kind: TestKind::Automated,
    },
    CogaRow {
        objective: "No memory reliance",
        feature: "One-step mode, task maps, visible answers, reminders",
        test: "crates/curbcut-understand/src/actions/tests.rs#every_listed_request_gets_a_preview",
        kind: TestKind::Automated,
    },
    CogaRow {
        objective: "Help and support",
        feature: "Provenance panel, help tile, human-handoff docs",
        test: "crates/curbcut-adapt/src/provenance/tests.rs#a_change_links_to_its_rule",
        kind: TestKind::Automated,
    },
    CogaRow {
        objective: "Adaptation",
        feature: "Profile graph, per-app layouts, learned simplifications",
        test: "crates/curbcut-adapt/src/profile/tests.rs#every_built_in_kind_maps_to_its_id",
        kind: TestKind::Automated,
    },
];

/// The published limitations, in the order the limitations page shows them.
pub const LIMITATIONS: &[&str] = &[
    "We cannot see protected/DRM content or secure desktops.",
    "We cannot reach into a native app's internals; we cover pixels, not code.",
    "Detection is probabilistic; we publish mAP and CER and a per-app confidence indicator.",
    "Simplification can be wrong; we check meaning and always show the original.",
    "We do not support every language equally; we publish the language matrix.",
    "Linux Wayland support depends on the portal; some distros are beta.",
    "We are not a screen reader and do not replace one.",
    "We are not a compliance tool and do not make apps conform.",
];

/// Renders the WCAG mapping table.
///
/// The Test cell is a link relative to `docs/`, so the same fragment is used by
/// `docs/A11Y_MAPPING.md` and `curbcut-cli vpat`.
pub fn render_wcag_table() -> String {
    let mut out = String::new();
    out.push_str("| Criterion | Level | CurbCut behavior | Status | Test |\n");
    out.push_str("| --- | --- | --- | --- | --- |\n");
    for row in WCAG {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            row.criterion,
            row.level,
            row.behavior,
            row.status_cell(),
            test_cell(row.test, row.kind)
        ));
    }
    out
}

/// Renders the COGA mapping table.
pub fn render_coga_table() -> String {
    let mut out = String::new();
    out.push_str("| COGA objective | CurbCut feature | Test |\n");
    out.push_str("| --- | --- | --- |\n");
    for row in COGA {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            row.objective,
            row.feature,
            test_cell(row.test, row.kind)
        ));
    }
    out
}

/// Renders the numbered limitations list.
pub fn render_limitations() -> String {
    let mut out = String::new();
    for (index, limitation) in LIMITATIONS.iter().enumerate() {
        out.push_str(&format!("{}. {}\n", index + 1, limitation));
    }
    out
}

/// Renders the draft VPAT table and its summary counts.
pub fn render_vpat_table() -> String {
    let mut out = String::new();
    out.push_str("| Criterion | Level | Result | Remarks |\n");
    out.push_str("| --- | --- | --- | --- |\n");
    for row in WCAG {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            row.criterion,
            row.level,
            row.status.vpat_result(),
            row.remark()
        ));
    }
    out.push('\n');
    out.push_str(&render_vpat_summary());
    out
}

/// Renders the draft VPAT: a banner, the table, and the review checklist.
pub fn draft_vpat() -> String {
    let mut out = String::new();
    out.push_str("# Draft VPAT: CurbCut AI\n\n");
    out.push_str(DRAFT_BANNER);
    out.push_str(&render_vpat_table());
    out.push_str(REVIEW_CHECKLIST);
    out
}

/// Renders the VPAT summary counts.
pub fn render_vpat_summary() -> String {
    let supports = WCAG
        .iter()
        .filter(|row| row.status == Status::Supports)
        .count();
    let partial = WCAG
        .iter()
        .filter(|row| row.status == Status::Partial)
        .count();
    let not_applicable = WCAG
        .iter()
        .filter(|row| row.status == Status::NotApplicable)
        .count();
    let not_assessed = WCAG
        .iter()
        .filter(|row| row.status == Status::NotAssessed)
        .count();
    format!(
        "**Draft summary:** {} Supports, {} Partially Supports, {} Not Applicable, \
         {} Not Assessed, across {} criteria.\n",
        supports,
        partial,
        not_applicable,
        not_assessed,
        WCAG.len()
    )
}

/// The draft banner, which is part of the published claim.
pub const DRAFT_BANNER: &str = "> **DRAFT.** Generated from `crates/curbcut-cli/src/vpat.rs`. \
This VPAT evaluates CurbCut's own UI only. It is not a conformance claim about any \
third-party application, and it has not been reviewed by a human.\n\n";

/// The human review checklist, which is part of the published claim.
pub const REVIEW_CHECKLIST: &str = "\n## Human review\n\n\
- [ ] Every row read by a human who has seen the product.\n\
- [ ] Every `Partially Supports` remark understood and accepted.\n\
- [ ] Every manual test in `docs/A11Y_AUDIT.md` executed and its result recorded.\n\
- [ ] The scope statement agreed: CurbCut's own UI, not a third-party app.\n\
- [ ] The draft banner removed only after the review is recorded.\n\n\
A VPAT is not published until every box is ticked and the review is recorded in\
 the pull request that publishes it.\n";

/// Renders a test cell: a link relative to `docs/`.
fn test_cell(test: &str, kind: TestKind) -> String {
    let target = doc_path(test);
    format!("[{}: `{}`]({})", kind.as_str(), test, target)
}

/// Rewrites a repository-relative test target as a `docs/`-relative link.
///
/// `docs/A11Y_AUDIT.md#x` becomes `A11Y_AUDIT.md#x`; `crates/...` becomes
/// `../crates/...`.
fn doc_path(test: &str) -> String {
    match test.split_once('#') {
        Some((path, fragment)) => format!("{}#{}", doc_base(path), fragment),
        None => doc_base(test),
    }
}

/// Rewrites the path half of a test target.
fn doc_base(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("docs/") {
        rest.to_string()
    } else {
        format!("../{path}")
    }
}

/// Upper-cases the first character of `text`.
fn capitalize(text: &str) -> String {
    match text.chars().next() {
        Some(first) => {
            let mut out: String = first.to_uppercase().collect();
            out.push_str(&text[first.len_utf8()..]);
            out
        }
        None => String::new(),
    }
}

/// The GitHub-style anchor for a heading.
///
/// Lower-case, drop punctuation, spaces to hyphens. Used to check that a manual
/// test link points at a real heading in `docs/A11Y_AUDIT.md`.
pub fn slug(text: &str) -> String {
    let mut out = String::new();
    let mut last_was_hyphen = false;
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_lowercase());
            last_was_hyphen = false;
        } else if (ch == ' ' || ch == '-' || ch == '_') && !last_was_hyphen {
            out.push('-');
            last_was_hyphen = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Splits a test target into its path and fragment.
///
/// Returns `(path, None)` when there is no fragment.
pub fn split_target(test: &str) -> (&str, Option<&str>) {
    match test.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (test, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_labels_are_stable() {
        assert_eq!(Level::A.as_str(), "A");
        assert_eq!(Level::Aa.as_str(), "AA");
        assert_eq!(Level::Aaa.as_str(), "AAA");
        assert_eq!(Level::Aa.to_string(), "AA");
    }

    #[test]
    fn status_labels_map_to_vpat_results() {
        assert_eq!(Status::Supports.label(), "Supports");
        assert_eq!(Status::Supports.vpat_result(), "Supports");
        assert_eq!(Status::Partial.label(), "Partially supports");
        assert_eq!(Status::Partial.vpat_result(), "Partially Supports");
        assert_eq!(Status::NotApplicable.vpat_result(), "Not Applicable");
        assert_eq!(Status::NotAssessed.vpat_result(), "Not Assessed");
    }

    #[test]
    fn test_kinds_are_named() {
        assert_eq!(TestKind::Automated.as_str(), "automated");
        assert_eq!(TestKind::Manual.as_str(), "manual");
    }

    #[test]
    fn a_row_id_is_the_numeric_prefix() {
        let row = &WCAG[0];
        assert_eq!(row.id(), "1.1.1");
        assert_eq!(row.criterion, "1.1.1 Non-text Content");
    }

    #[test]
    fn every_row_names_a_test() {
        for row in WCAG {
            assert!(!row.test.is_empty(), "{}", row.criterion);
            assert!(
                row.test.starts_with("crates/") || row.test.starts_with("docs/"),
                "{}: {}",
                row.criterion,
                row.test
            );
            let (path, fragment) = split_target(row.test);
            assert!(!path.is_empty(), "{}", row.criterion);
            assert!(fragment.is_some(), "{}: {}", row.criterion, row.test);
        }
        for row in COGA {
            assert!(!row.test.is_empty(), "{}", row.objective);
            assert!(
                row.test.starts_with("crates/") || row.test.starts_with("docs/"),
                "{}: {}",
                row.objective,
                row.test
            );
        }
    }

    #[test]
    fn criterion_ids_are_unique_and_ordered() {
        let mut ids: Vec<&str> = WCAG.iter().map(MappingRow::id).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "a criterion appears twice");
    }

    #[test]
    fn the_status_cell_appends_the_qualifier() {
        let row = &WCAG[0];
        assert_eq!(
            row.status_cell(),
            "Partially supports; only for detected icons"
        );
        let plain = WCAG
            .iter()
            .find(|row| row.criterion.starts_with("1.3.2"))
            .unwrap();
        assert_eq!(plain.status_cell(), "Partially supports");
    }

    #[test]
    fn a_remark_names_the_test_and_the_qualifier() {
        let row = &WCAG[0];
        let remark = row.remark();
        assert!(remark.contains("Only for detected icons."), "{remark}");
        assert!(remark.contains("automated test"), "{remark}");
        assert!(remark.contains(row.test), "{remark}");
        assert!(remark.ends_with('.'), "{remark}");
    }

    #[test]
    fn the_tables_have_a_row_per_entry() {
        let wcag = render_wcag_table();
        assert!(wcag.starts_with("| Criterion | Level | CurbCut behavior | Status | Test |"));
        for row in WCAG {
            assert!(
                wcag.contains(&format!("| {} |", row.criterion)),
                "{}",
                row.criterion
            );
        }
        let coga = render_coga_table();
        for row in COGA {
            assert!(
                coga.contains(&format!("| {} |", row.objective)),
                "{}",
                row.objective
            );
        }
    }

    #[test]
    fn the_limitations_are_numbered_from_one() {
        let rendered = render_limitations();
        let lines: Vec<&str> = rendered.lines().collect();
        assert_eq!(lines.len(), LIMITATIONS.len());
        for (index, line) in lines.iter().enumerate() {
            assert!(line.starts_with(&format!("{}. ", index + 1)), "{line}");
        }
    }

    #[test]
    fn the_vpat_summary_counts_every_row() {
        let summary = render_vpat_summary();
        let total = WCAG.len();
        assert!(summary.contains(&format!("across {total} criteria")));
        let supports = WCAG
            .iter()
            .filter(|row| row.status == Status::Supports)
            .count();
        assert!(summary.contains(&format!("{supports} Supports")));
        assert!(render_vpat_table().contains("Partially Supports"));
    }

    #[test]
    fn the_draft_vpat_carries_the_banner_and_the_checklist() {
        let draft = draft_vpat();
        assert!(draft.starts_with("# Draft VPAT: CurbCut AI"));
        assert!(draft.contains("DRAFT"));
        assert!(draft.contains("Human review"));
        assert!(draft.contains(&render_vpat_table()));
    }

    #[test]
    fn doc_paths_are_relative_to_docs() {
        assert_eq!(
            doc_path("crates/curbcut-a11y/tests/differential.rs#x"),
            "../crates/curbcut-a11y/tests/differential.rs#x"
        );
        assert_eq!(
            doc_path("docs/A11Y_AUDIT.md#143-contrast-minimum"),
            "A11Y_AUDIT.md#143-contrast-minimum"
        );
        assert_eq!(doc_path("crates/x.rs"), "../crates/x.rs");
    }

    #[test]
    fn slugs_match_the_audit_headings() {
        assert_eq!(slug("1.4.3 Contrast (Minimum)"), "143-contrast-minimum");
        assert_eq!(slug("2.2.2 Pause, Stop, Hide"), "222-pause-stop-hide");
        assert_eq!(slug("1.1.1 Non-text Content"), "111-non-text-content");
        assert_eq!(slug("  Trimmed  Heading  "), "trimmed-heading");
    }

    #[test]
    fn split_targets_handles_a_missing_fragment() {
        assert_eq!(split_target("crates/x.rs"), ("crates/x.rs", None));
        assert_eq!(
            split_target("crates/x.rs#test"),
            ("crates/x.rs", Some("test"))
        );
    }
}
