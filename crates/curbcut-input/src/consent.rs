//! Action consent flow.
//!
//! Nothing happens in another application because the overlay decided it should.
//! Every action class needs a consent the user gave in this session, and the
//! consent text states the consequence before the user chooses. See
//! [`../../../docs/CONSENT.md`](../../../docs/CONSENT.md).
//!
//! Four rules hold:
//!
//! - **No action without consent.** [`ConsentGate::authorize`] returns
//!   [`ConsentError::NotGranted`] unless a receipt for that action class exists.
//! - **The consequence comes first.** [`ConsentPrompt::consequence`] is a fixed
//!   sentence per action class; it never quotes the target's label.
//! - **A receipt is scoped.** [`ConsentScope::Once`] is consumed by the action
//!   it authorized; [`ConsentScope::Session`] lasts until the gate is cleared.
//! - **The log records action and role only.** [`ConsentRecord`] carries a
//!   sequence number, an action class, a target role, and an outcome — never a
//!   label, a value, or screen text.

use std::collections::VecDeque;
use std::fmt;

/// How many consent records the gate retains.
pub const MAX_RECORDS: usize = 4096;

/// A class of action the user can consent to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ActionClass {
    /// Move focus or activate a link.
    Navigate,
    /// Change a value in a control.
    Edit,
    /// Send a form or commit a change.
    Submit,
    /// Close a dialog, banner, or window.
    Dismiss,
    /// Scroll the content.
    Scroll,
    /// Invoke a control's default action.
    Invoke,
    /// Run a global shortcut.
    Shortcut,
    /// Change the system: volume, brightness, notifications.
    System,
}

impl ActionClass {
    /// The stable string form used in logs and telemetry.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Navigate => "navigate",
            Self::Edit => "edit",
            Self::Submit => "submit",
            Self::Dismiss => "dismiss",
            Self::Scroll => "scroll",
            Self::Invoke => "invoke",
            Self::Shortcut => "shortcut",
            Self::System => "system",
        }
    }

    /// The plain-language consequence of the action.
    ///
    /// It names what the application will do, not what is on the screen.
    pub fn consequence(&self) -> &'static str {
        match self {
            Self::Navigate => "The application will move focus or open the link you chose.",
            Self::Edit => "The application will change the value of the control you chose.",
            Self::Submit => "The application will send the form or commit the change you chose.",
            Self::Dismiss => "The application will close the dialog, banner, or window you chose.",
            Self::Scroll => "The application will scroll the content you chose.",
            Self::Invoke => "The application will activate the control you chose.",
            Self::Shortcut => "CurbCut will run the global shortcut you chose.",
            Self::System => "CurbCut will change a system setting, such as volume or brightness.",
        }
    }
}

impl fmt::Display for ActionClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The kind of control an action targets.
///
/// It is a role, not a label: the log can say "a button" but never what the
/// button says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TargetRole {
    /// A button.
    Button,
    /// A link.
    Link,
    /// A text field.
    TextBox,
    /// A menu item.
    MenuItem,
    /// A checkbox or switch.
    Checkbox,
    /// A slider.
    Slider,
    /// A tab.
    Tab,
    /// Anything else.
    Generic,
}

impl TargetRole {
    /// The stable string form used in logs and telemetry.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Link => "link",
            Self::TextBox => "textbox",
            Self::MenuItem => "menu_item",
            Self::Checkbox => "checkbox",
            Self::Slider => "slider",
            Self::Tab => "tab",
            Self::Generic => "generic",
        }
    }
}

impl fmt::Display for TargetRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Proof that the user consented to one action class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConsentToken(u64);

impl ConsentToken {
    fn new(id: u64) -> Self {
        Self(id)
    }

    /// The token's numeric id.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// How long a granted consent lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsentScope {
    /// The receipt authorizes one action, then is consumed.
    Once,
    /// The receipt lasts until [`ConsentGate::clear`].
    Session,
}

impl ConsentScope {
    /// The stable string form used in logs and telemetry.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Session => "session",
        }
    }
}

impl fmt::Display for ConsentScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A pending request for consent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentPrompt {
    token: ConsentToken,
    action: ActionClass,
    target_role: TargetRole,
}

impl ConsentPrompt {
    fn new(token: ConsentToken, action: ActionClass, target_role: TargetRole) -> Self {
        Self {
            token,
            action,
            target_role,
        }
    }

    /// The action class being asked about.
    pub fn action(&self) -> ActionClass {
        self.action
    }

    /// The role of the target.
    pub fn target_role(&self) -> TargetRole {
        self.target_role
    }

    /// The plain-language question: what will happen.
    pub fn question(&self) -> String {
        format!("{} {}", self.action.consequence(), "Allow it?",)
    }

    /// The consequence sentence alone.
    pub fn consequence(&self) -> &'static str {
        self.action.consequence()
    }

    /// The token that [`ConsentGate::grant`] or [`ConsentGate::deny`] takes.
    pub fn token(&self) -> ConsentToken {
        self.token
    }
}

/// Why an action was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentError {
    /// No receipt exists for the action class.
    NotGranted,
    /// The token does not match a pending prompt.
    UnknownToken,
}

impl fmt::Display for ConsentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotGranted => f.write_str("no consent for this action"),
            Self::UnknownToken => f.write_str("unknown consent token"),
        }
    }
}

impl std::error::Error for ConsentError {}

/// What happened to a consent request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentOutcome {
    /// The user allowed it.
    Granted,
    /// The user refused it.
    Denied,
}

impl ConsentOutcome {
    /// The stable string form used in logs and telemetry.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Denied => "denied",
        }
    }
}

impl fmt::Display for ConsentOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A granted or refused consent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentReceipt {
    action: ActionClass,
    target_role: TargetRole,
    outcome: ConsentOutcome,
    scope: ConsentScope,
    sequence: u64,
}

impl ConsentReceipt {
    /// The action class.
    pub fn action(&self) -> ActionClass {
        self.action
    }

    /// The role of the target.
    pub fn target_role(&self) -> TargetRole {
        self.target_role
    }

    /// Whether it was granted.
    pub fn outcome(&self) -> ConsentOutcome {
        self.outcome
    }

    /// The scope the user chose.
    pub fn scope(&self) -> ConsentScope {
        self.scope
    }

    /// Whether this receipt can authorize an action.
    pub fn is_granted(&self) -> bool {
        self.outcome == ConsentOutcome::Granted
    }
}

/// Proof that one action is allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsentProof {
    action: ActionClass,
    target_role: TargetRole,
    sequence: u64,
}

impl ConsentProof {
    /// The action class.
    pub fn action(&self) -> ActionClass {
        self.action
    }

    /// The role of the target.
    pub fn target_role(&self) -> TargetRole {
        self.target_role
    }

    /// The sequence number of the receipt that authorized it.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
}

/// One consent decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsentRecord {
    /// Position in the decision sequence, starting at 1.
    pub sequence: u64,
    /// The action class.
    pub action: ActionClass,
    /// The role of the target.
    pub target_role: TargetRole,
    /// What the user decided.
    pub outcome: ConsentOutcome,
    /// The scope the user chose.
    pub scope: ConsentScope,
}

/// Gates every action behind a consent the user gave in this session.
#[derive(Debug)]
pub struct ConsentGate {
    next_token: u64,
    next_sequence: u64,
    pending: Vec<ConsentPrompt>,
    receipts: Vec<ConsentReceipt>,
    log: VecDeque<ConsentRecord>,
    dropped: u64,
}

impl Default for ConsentGate {
    fn default() -> Self {
        Self {
            next_token: 0,
            next_sequence: 1,
            pending: Vec::new(),
            receipts: Vec::new(),
            log: VecDeque::new(),
            dropped: 0,
        }
    }
}

impl ConsentGate {
    /// Creates an empty gate.
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks for consent for one action.
    ///
    /// The prompt carries the consequence sentence; the caller shows it and then
    /// calls [`ConsentGate::grant`] or [`ConsentGate::deny`].
    pub fn prompt(&mut self, action: ActionClass, target_role: TargetRole) -> ConsentPrompt {
        let token = ConsentToken::new(self.next_token);
        self.next_token = self.next_token.wrapping_add(1);
        let prompt = ConsentPrompt::new(token, action, target_role);
        self.pending.push(prompt.clone());
        prompt
    }

    /// Grants a pending prompt.
    pub fn grant(&mut self, token: ConsentToken, scope: ConsentScope) -> Option<ConsentReceipt> {
        self.resolve(token, ConsentOutcome::Granted, scope)
    }

    /// Refuses a pending prompt.
    pub fn deny(&mut self, token: ConsentToken) -> Option<ConsentReceipt> {
        self.resolve(token, ConsentOutcome::Denied, ConsentScope::Once)
    }

    fn resolve(
        &mut self,
        token: ConsentToken,
        outcome: ConsentOutcome,
        scope: ConsentScope,
    ) -> Option<ConsentReceipt> {
        let index = self
            .pending
            .iter()
            .position(|prompt| prompt.token == token)?;
        let prompt = self.pending.remove(index);
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        self.push_record(ConsentRecord {
            sequence,
            action: prompt.action,
            target_role: prompt.target_role,
            outcome,
            scope,
        });
        let receipt = ConsentReceipt {
            action: prompt.action,
            target_role: prompt.target_role,
            outcome,
            scope,
            sequence,
        };
        if outcome == ConsentOutcome::Granted {
            self.receipts.push(receipt.clone());
        }
        Some(receipt)
    }

    fn push_record(&mut self, record: ConsentRecord) {
        if self.log.len() >= MAX_RECORDS {
            self.log.pop_front();
            self.dropped = self.dropped.wrapping_add(1);
        }
        self.log.push_back(record);
    }

    /// Authorizes one action.
    ///
    /// Returns [`ConsentError::NotGranted`] when no granted receipt exists for
    /// the action class. A [`ConsentScope::Once`] receipt is consumed; a
    /// [`ConsentScope::Session`] receipt stays available.
    pub fn authorize(
        &mut self,
        action: ActionClass,
        target_role: TargetRole,
    ) -> Result<ConsentProof, ConsentError> {
        let index = self
            .receipts
            .iter()
            .position(|receipt| receipt.action == action && receipt.is_granted())
            .ok_or(ConsentError::NotGranted)?;
        let receipt = self.receipts.remove(index);
        if receipt.scope == ConsentScope::Session {
            self.receipts.push(receipt.clone());
        }
        Ok(ConsentProof {
            action,
            target_role,
            sequence: receipt.sequence,
        })
    }

    /// Whether an action class currently has a granted receipt.
    pub fn is_granted(&self, action: ActionClass) -> bool {
        self.receipts
            .iter()
            .any(|receipt| receipt.action == action && receipt.is_granted())
    }

    /// Drops every receipt and pending prompt. The log is kept.
    pub fn clear(&mut self) {
        self.pending.clear();
        self.receipts.clear();
    }

    /// Every retained decision, oldest first.
    pub fn log(&self) -> &VecDeque<ConsentRecord> {
        &self.log
    }

    /// The number of decisions dropped to keep the log bounded.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// The number of pending prompts.
    pub fn pending(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests;
