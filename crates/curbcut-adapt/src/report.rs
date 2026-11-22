//! Right to explanation: a plain-language report of what CurbCut did.
//!
//! A user can ask what the overlay has been doing. [`crate::report::Report`] answers with one
//! plain-language entry per action: what happened, why, and the original value
//! it replaced. See
//! [`../../../docs/RIGHT_TO_EXPLANATION.md`](../../../docs/RIGHT_TO_EXPLANATION.md).
//!
//! Three rules hold:
//!
//! - **Every action is explained or flagged.** An entry with no rationale is
//!   rendered with "no reason was recorded", and
//!   [`crate::report::Report::missing_rationale`] lists it, so an unexplained action cannot
//!   pass silently.
//! - **The report is readable.** [`crate::report::Report::grade_level`] measures the export
//!   with the Flesch-Kincaid grade and [`crate::report::Report::is_readable_at`] compares it
//!   with a target, so "readable at a grade-6 level" is a measured outcome
//!   rather than an intention.
//! - **The export carries no other app's content.** The report renders an
//!   original through [`crate::provenance::OriginalValue::render`], which shows a setting value
//!   because it is user-chosen, and shows content-derived text only as its
//!   length and identity hash because the text was never retained. An action id
//!   is a fixed action name, never a label.
//!
//! The module is pure: no I/O, no clock, and no panics. The caller supplies
//! `generated_at_ms`.

use std::collections::{HashMap, HashSet, VecDeque};

use curbcut_core::id::RuleId;
use curbcut_core::types::Rationale;
use curbcut_understand::reading::flesch_kincaid_grade;

use crate::confirm::{ActionId, ActionLog, Outcome};
use crate::provenance::{OriginalValue, ProvenancePanel};

/// One action in the report.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportEntry {
    action: ActionId,
    rule: RuleId,
    outcome: Option<Outcome>,
    original: OriginalValue,
    changed_at_ms: Option<u64>,
    reason: Option<String>,
}

impl ReportEntry {
    /// Creates an entry.
    pub fn new(
        action: ActionId,
        rule: RuleId,
        outcome: Option<Outcome>,
        original: OriginalValue,
        changed_at_ms: Option<u64>,
        reason: Option<String>,
    ) -> Self {
        Self {
            action,
            rule,
            outcome,
            original,
            changed_at_ms,
            reason,
        }
    }

    /// The action.
    pub fn action(&self) -> &ActionId {
        &self.action
    }

    /// The rule that authorised it.
    pub fn rule(&self) -> &RuleId {
        &self.rule
    }

    /// What happened, or `None` when the action log did not record an outcome.
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// The original value the action replaced.
    pub fn original(&self) -> &OriginalValue {
        &self.original
    }

    /// When the action ran, in milliseconds, when the provenance panel has it.
    pub fn changed_at_ms(&self) -> Option<u64> {
        self.changed_at_ms
    }

    /// The reason, when a rationale was supplied for the rule.
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// Whether the entry has a non-empty reason.
    pub fn is_explained(&self) -> bool {
        self.reason
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
    }

    /// The plain-language verb for the outcome.
    pub fn what_happened(&self) -> &'static str {
        match self.outcome {
            Some(Outcome::Applied) => "CurbCut applied",
            Some(Outcome::Dismissed) => "You dismissed",
            Some(Outcome::Undone) => "CurbCut applied, then undid",
            None => "The outcome of",
        }
    }

    /// The one-sentence summary.
    pub fn sentence(&self) -> String {
        match self.outcome {
            Some(Outcome::Applied) => format!("CurbCut applied the {} action.", self.action),
            Some(Outcome::Dismissed) => format!("You dismissed the {} action.", self.action),
            Some(Outcome::Undone) => {
                format!("CurbCut applied the {} action, then undid it.", self.action)
            }
            None => format!(
                "The outcome of the {} action was not recorded.",
                self.action
            ),
        }
    }

    /// The full plain-language explanation: what, why, and the original.
    pub fn explanation(&self) -> String {
        let mut reason = match self.reason() {
            Some(text) if !text.trim().is_empty() => text.trim().to_string(),
            _ => "no reason was recorded".to_string(),
        };
        if !reason.ends_with('.') {
            reason.push('.');
        }
        let original = self.original.render();
        let when = self
            .changed_at_ms
            .map_or_else(|| "time not recorded".to_string(), |ms| format!("{ms} ms"));
        format!(
            "{} Why: {reason} Original: {original}. When: {when}.",
            self.sentence()
        )
    }
}

/// One action's outcome, as the log recorded it.
struct LogSlot {
    rule: RuleId,
    outcome: Outcome,
}

/// One action's original value, as the provenance panel recorded it.
struct PanelSlot {
    rule: RuleId,
    original: OriginalValue,
    changed_at_ms: u64,
}

/// A plain-language report of every action.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    entries: Vec<ReportEntry>,
    generated_at_ms: u64,
}

impl Report {
    /// Creates a report from entries.
    pub fn new(entries: Vec<ReportEntry>, generated_at_ms: u64) -> Self {
        Self {
            entries,
            generated_at_ms,
        }
    }

    /// Builds a report from the action log, the provenance panel, and the
    /// rationales.
    ///
    /// The two logs are independent, so they are paired by action id in the
    /// order each recorded them: the first `dim` in the log is the first `dim`
    /// in the panel. An action that appears in only one of them is still
    /// reported, with the missing half left blank, so the report never drops an
    /// action. `generated_at_ms` is supplied by the caller.
    pub fn from_sources(
        log: &ActionLog,
        panel: &ProvenancePanel,
        rationales: &[Rationale],
        generated_at_ms: u64,
    ) -> Self {
        let mut order: Vec<ActionId> = Vec::new();
        let mut seen: HashSet<ActionId> = HashSet::new();
        let mut log_slots: HashMap<ActionId, VecDeque<LogSlot>> = HashMap::new();
        let mut panel_slots: HashMap<ActionId, VecDeque<PanelSlot>> = HashMap::new();

        for record in log.records() {
            if seen.insert(record.action().clone()) {
                order.push(record.action().clone());
            }
            log_slots
                .entry(record.action().clone())
                .or_default()
                .push_back(LogSlot {
                    rule: record.rule().clone(),
                    outcome: record.outcome(),
                });
        }
        for entry in panel.entries() {
            if seen.insert(entry.action().clone()) {
                order.push(entry.action().clone());
            }
            panel_slots
                .entry(entry.action().clone())
                .or_default()
                .push_back(PanelSlot {
                    rule: entry.rule().clone(),
                    original: entry.original().clone(),
                    changed_at_ms: entry.changed_at_ms(),
                });
        }

        // The first rationale for a rule wins, so a report cannot change its
        // reasoning because a later rule offered a different explanation.
        let mut reasons: HashMap<RuleId, String> = HashMap::new();
        for rationale in rationales {
            reasons
                .entry(rationale.rule.clone())
                .or_insert_with(|| rationale.text.clone());
        }

        let mut entries = Vec::new();
        for action in order {
            let mut outcomes = log_slots.remove(&action).unwrap_or_default();
            let mut originals = panel_slots.remove(&action).unwrap_or_default();
            let count = outcomes.len().max(originals.len());
            for _ in 0..count {
                let log_slot = outcomes.pop_front();
                let panel_slot = originals.pop_front();
                let (rule, outcome, original, changed_at_ms) = match (log_slot, panel_slot) {
                    (Some(log_slot), Some(panel_slot)) => (
                        panel_slot.rule.clone(),
                        Some(log_slot.outcome),
                        Some(panel_slot.original.clone()),
                        Some(panel_slot.changed_at_ms),
                    ),
                    (Some(log_slot), None) => {
                        (log_slot.rule.clone(), Some(log_slot.outcome), None, None)
                    }
                    (None, Some(panel_slot)) => (
                        panel_slot.rule.clone(),
                        None,
                        Some(panel_slot.original.clone()),
                        Some(panel_slot.changed_at_ms),
                    ),
                    // `count` is the larger of the two lengths, so one of the
                    // two queues has a slot here.
                    (None, None) => break,
                };
                let reason = reasons.get(&rule).cloned();
                entries.push(ReportEntry {
                    action: action.clone(),
                    rule,
                    outcome,
                    original: original.unwrap_or(OriginalValue::None),
                    changed_at_ms,
                    reason,
                });
            }
        }

        Self {
            entries,
            generated_at_ms,
        }
    }

    /// Every entry, oldest first.
    pub fn entries(&self) -> &[ReportEntry] {
        &self.entries
    }

    /// How many entries there are.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the report is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// When the report was generated, in milliseconds, as the caller supplied.
    pub fn generated_at_ms(&self) -> u64 {
        self.generated_at_ms
    }

    /// The actions with no recorded reason.
    pub fn missing_rationale(&self) -> Vec<&ActionId> {
        self.entries
            .iter()
            .filter(|entry| !entry.is_explained())
            .map(ReportEntry::action)
            .collect()
    }

    /// Whether every entry has a reason.
    pub fn is_explained(&self) -> bool {
        self.entries.iter().all(ReportEntry::is_explained)
    }

    /// The Flesch-Kincaid grade level of the report body, or `None` when the
    /// script is unsupported.
    ///
    /// The body is the header and the entries; the footer is excluded because it
    /// quotes the grade, which would measure its own label.
    pub fn grade_level(&self) -> Option<f32> {
        flesch_kincaid_grade(&self.body_text())
    }

    /// Whether the export reads at or below `grade`.
    ///
    /// `None` when the grade cannot be computed, which is not readable.
    pub fn is_readable_at(&self, grade: f32) -> bool {
        self.grade_level().is_some_and(|level| level <= grade)
    }

    /// The plain-text export.
    pub fn to_plain_text(&self) -> String {
        let mut text = self.body_text();
        text.push_str(&format!(
            "\nActions listed: {}\nReading level: {}\nReport generated at: {} ms\n",
            self.entries.len(),
            self.grade_label(),
            self.generated_at_ms,
        ));
        text
    }

    /// The Markdown export.
    pub fn to_markdown(&self) -> String {
        let mut text = String::new();
        text.push_str("# CurbCut action report\n\n");
        text.push_str(
            "This report lists what CurbCut did, why it did it, and what it saw.\n\
             It does not include the text of other apps.\n",
        );
        for entry in &self.entries {
            text.push_str(&format!("\n- {}\n", entry.explanation()));
        }
        text.push_str(&format!(
            "\n**Actions listed:** {}\n\n**Reading level:** {}\n\n**Report generated at:** {} ms\n",
            self.entries.len(),
            self.grade_label(),
            self.generated_at_ms,
        ));
        text
    }

    /// The header and the entries, without the measured footer.
    fn body_text(&self) -> String {
        let mut text = String::new();
        text.push_str("CurbCut action report\n\n");
        text.push_str(
            "This report lists what CurbCut did, why it did it, and what it saw.\n\
             It does not include the text of other apps.\n",
        );
        for (index, entry) in self.entries.iter().enumerate() {
            text.push_str(&format!("\n{}. {}\n", index + 1, entry.explanation()));
        }
        text
    }

    /// The grade as a label, or a note that it could not be measured.
    fn grade_label(&self) -> String {
        self.grade_level()
            .map_or_else(|| "not measured".to_string(), |level| format!("{level:.1}"))
    }
}

#[cfg(test)]
mod tests;
