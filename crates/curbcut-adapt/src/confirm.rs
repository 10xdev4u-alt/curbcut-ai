//! Confirmation and undo for destructive actions.
//!
//! A destructive action is confirmed in plain language before it runs, and the
//! outcome is recorded. Undo is offered only when the action can be undone.
//!
//! Three rules hold:
//!
//! - **The consequence is stated.** [`Confirmation::consequence`] is the
//!   plain-language sentence the user reads; it describes what will happen,
//!   never what is on screen.
//! - **Undo is offered only when possible.** [`Confirmation::offers_undo`] is
//!   the single source of truth, and [`ActionLog::undo_last`] refuses to undo
//!   an action that was not undoable.
//! - **The outcome is recorded.** Every confirmation is appended to an
//!   [`ActionLog`] with its [`Outcome`], so the user can see what happened.
//!
//! The log holds action ids, rule references, and outcomes. It never holds
//! screen content, OCR text, or accessibility strings.

use std::fmt;

use curbcut_core::id::RuleId;

/// An action that changes the user's app or settings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActionId(String);

impl ActionId {
    /// Creates an action id from a slug.
    pub fn new(slug: &str) -> Self {
        Self(slug.to_lowercase())
    }

    /// The string form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ActionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A plain-language confirmation for a destructive action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirmation {
    action: ActionId,
    consequence: String,
    undoable: bool,
}

impl Confirmation {
    /// Creates a confirmation.
    pub fn new(action: ActionId, consequence: impl Into<String>, undoable: bool) -> Self {
        Self {
            action,
            consequence: consequence.into(),
            undoable,
        }
    }

    /// The action that needs confirming.
    pub fn action(&self) -> &ActionId {
        &self.action
    }

    /// The plain-language consequence.
    pub fn consequence(&self) -> &str {
        &self.consequence
    }

    /// Whether the action can be undone.
    pub fn is_undoable(&self) -> bool {
        self.undoable
    }

    /// Whether the interface should offer undo.
    ///
    /// Undo is offered only when the action can be undone.
    pub fn offers_undo(&self) -> bool {
        self.undoable
    }
}

/// The user's answer to a confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Decision {
    /// The user confirmed; the action ran.
    Confirmed,
    /// The user dismissed; nothing ran.
    Dismissed,
}

/// What happened to a confirmed action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// The action ran.
    Applied,
    /// The user dismissed it.
    Dismissed,
    /// The action ran and was undone.
    Undone,
}

/// One entry in the action log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRecord {
    action: ActionId,
    rule: RuleId,
    outcome: Outcome,
    undoable: bool,
}

impl ActionRecord {
    /// The action.
    pub fn action(&self) -> &ActionId {
        &self.action
    }

    /// The rule that authorised it.
    pub fn rule(&self) -> &RuleId {
        &self.rule
    }

    /// The outcome.
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }

    /// Whether the action was undoable.
    pub fn is_undoable(&self) -> bool {
        self.undoable
    }

    /// Whether the action was undone.
    pub fn is_undone(&self) -> bool {
        self.outcome == Outcome::Undone
    }
}

/// An append-only log of outcomes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionLog {
    records: Vec<ActionRecord>,
}

impl ActionLog {
    /// Creates an empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many records there are.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Every record, oldest first.
    pub fn records(&self) -> &[ActionRecord] {
        &self.records
    }

    /// The most recent record.
    pub fn last(&self) -> Option<&ActionRecord> {
        self.records.last()
    }

    /// Records the outcome of a confirmation.
    pub fn record(&mut self, confirmation: &Confirmation, rule: RuleId, decision: Decision) {
        let outcome = match decision {
            Decision::Confirmed => Outcome::Applied,
            Decision::Dismissed => Outcome::Dismissed,
        };
        self.records.push(ActionRecord {
            action: confirmation.action.clone(),
            rule,
            outcome,
            // Read the method, not the field, so the log cannot disagree with
            // what the interface offers.
            undoable: confirmation.offers_undo(),
        });
    }

    /// Undoes the most recent applied, undoable, not-yet-undone action.
    ///
    /// Returns its id, or `None` when there is nothing to undo.
    pub fn undo_last(&mut self) -> Option<ActionId> {
        let index = self
            .records
            .iter()
            .rposition(|record| record.outcome == Outcome::Applied && record.undoable)?;
        self.records[index].outcome = Outcome::Undone;
        Some(self.records[index].action.clone())
    }

    /// Whether [`ActionLog::undo_last`] would undo something.
    pub fn can_undo(&self) -> bool {
        self.records
            .iter()
            .any(|record| record.outcome == Outcome::Applied && record.undoable)
    }

    /// How many actions ran and were not undone.
    pub fn applied_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.outcome == Outcome::Applied)
            .count()
    }

    /// How many actions were undone.
    pub fn undone_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.outcome == Outcome::Undone)
            .count()
    }

    /// Drops every record.
    pub fn clear(&mut self) {
        self.records.clear();
    }
}

#[cfg(test)]
mod tests;
