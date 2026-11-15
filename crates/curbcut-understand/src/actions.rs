//! Destructive-action preview and confirmation.
//!
//! Before anything is performed, the catalog says whether the request is
//! destructive and shows a plain preview of the consequence. The catalog has no
//! "perform" path: a caller that wants to act must first get a
//! [`Confirmation::Required`] and show the preview. See
//! [`../../../docs/DESTRUCTIVE_ACTIONS.md`](../../../docs/DESTRUCTIVE_ACTIONS.md).
//!
//! Three rules hold:
//!
//! - **The list is data, versioned, and auditable.**
//!   [`ActionCatalog::version`] and [`ActionCatalog::actions`] expose it.
//! - **A preview is shown for every listed action.**
//!   [`ActionCatalog::preview`] returns a summary for a detected action, and
//!   [`ActionCatalog::detect`] is the only way to reach it.
//! - **No action is performed without confirmation.** There is no perform
//!   method; [`Confirmation::Required`] carries the preview and
//!   [`Confirmation::can_proceed`] is false until the user accepts it.

use std::fmt;

/// The catalog version. Bump it when the shipped list changes.
pub const CATALOG_VERSION: &str = "2026-11-01";

/// What kind of consequence an action has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DestructiveKind {
    /// Removes something.
    Delete,
    /// Replaces existing content.
    Overwrite,
    /// Sends or publishes something.
    Send,
    /// Spends money.
    Purchase,
    /// Ends a session or subscription.
    Terminate,
    /// Takes a permission away.
    Revoke,
    /// Destroys a storage device.
    Format,
    /// Makes something visible to others.
    Publish,
}

impl DestructiveKind {
    /// Every supported kind, in display order.
    pub const ALL: [DestructiveKind; 8] = [
        Self::Delete,
        Self::Overwrite,
        Self::Send,
        Self::Purchase,
        Self::Terminate,
        Self::Revoke,
        Self::Format,
        Self::Publish,
    ];

    /// The stable name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Delete => "delete",
            Self::Overwrite => "overwrite",
            Self::Send => "send",
            Self::Purchase => "purchase",
            Self::Terminate => "terminate",
            Self::Revoke => "revoke",
            Self::Format => "format",
            Self::Publish => "publish",
        }
    }

    /// The human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Delete => "Delete",
            Self::Overwrite => "Overwrite",
            Self::Send => "Send",
            Self::Purchase => "Purchase",
            Self::Terminate => "Terminate",
            Self::Revoke => "Revoke",
            Self::Format => "Format",
            Self::Publish => "Publish",
        }
    }
}

impl fmt::Display for DestructiveKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One destructive action in the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestructiveAction {
    id: &'static str,
    kind: DestructiveKind,
    phrase: &'static str,
    consequence: &'static str,
    reversible: bool,
    triggers: &'static [&'static str],
}

impl DestructiveAction {
    /// Creates an action.
    pub const fn new(
        id: &'static str,
        kind: DestructiveKind,
        phrase: &'static str,
        consequence: &'static str,
        reversible: bool,
        triggers: &'static [&'static str],
    ) -> Self {
        Self {
            id,
            kind,
            phrase,
            consequence,
            reversible,
            triggers,
        }
    }

    /// The stable id.
    pub fn id(&self) -> &'static str {
        self.id
    }

    /// The consequence kind.
    pub fn kind(&self) -> DestructiveKind {
        self.kind
    }

    /// What the action does, in plain words.
    pub fn phrase(&self) -> &str {
        self.phrase
    }

    /// What happens as a result.
    pub fn consequence(&self) -> &str {
        self.consequence
    }

    /// Whether the action can be undone.
    pub fn is_reversible(&self) -> bool {
        self.reversible
    }

    /// The words and phrases that detect it.
    pub fn triggers(&self) -> &[&'static str] {
        self.triggers
    }

    /// The plain preview sentence.
    pub fn summary(&self) -> String {
        let mut text = format!("This will {}. {}", self.phrase, self.consequence);
        if self.reversible {
            text.push_str(" You can undo this.");
        } else {
            text.push_str(" This cannot be undone.");
        }
        text
    }
}

/// The versioned, auditable action catalog.
#[derive(Debug, Clone)]
pub struct ActionCatalog {
    actions: Vec<DestructiveAction>,
    version: &'static str,
}

impl Default for ActionCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionCatalog {
    /// Creates a catalog with the shipped list.
    pub fn new() -> Self {
        Self {
            actions: builtin_actions(),
            version: CATALOG_VERSION,
        }
    }

    /// Creates a catalog with no actions.
    pub fn empty() -> Self {
        Self {
            actions: Vec::new(),
            version: CATALOG_VERSION,
        }
    }

    /// The catalog version.
    pub fn version(&self) -> &'static str {
        self.version
    }

    /// The auditable list.
    pub fn actions(&self) -> &[DestructiveAction] {
        &self.actions
    }

    /// How many actions there are.
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Whether the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Looks an action up by id.
    pub fn find(&self, id: &str) -> Option<&DestructiveAction> {
        self.actions.iter().find(|action| action.id == id)
    }

    /// Adds an action, replacing one with the same id.
    pub fn add(&mut self, action: DestructiveAction) -> Option<DestructiveAction> {
        if let Some(slot) = self
            .actions
            .iter_mut()
            .find(|existing| existing.id == action.id)
        {
            return Some(std::mem::replace(slot, action));
        }
        self.actions.push(action);
        None
    }

    /// Removes an action by id, returning it.
    pub fn remove(&mut self, id: &str) -> Option<DestructiveAction> {
        let index = self.actions.iter().position(|action| action.id == id)?;
        Some(self.actions.remove(index))
    }

    /// Detects the most specific action a text asks for.
    ///
    /// When several triggers match, the longest one wins, so
    /// "permanently delete" is reported as a permanent delete rather than a
    /// plain delete. Ties break on catalog order.
    pub fn detect(&self, text: &str) -> Option<&DestructiveAction> {
        let lower = text.to_lowercase();
        let mut best: Option<(usize, &DestructiveAction)> = None;
        for action in &self.actions {
            for trigger in action.triggers {
                if !contains_trigger(&lower, trigger) {
                    continue;
                }
                if best.is_none_or(|(best_len, _)| trigger.len() > best_len) {
                    best = Some((trigger.len(), action));
                }
            }
        }
        best.map(|(_, action)| action)
    }

    /// The preview for a text, or `None` when nothing destructive is detected.
    pub fn preview(&self, text: &str) -> Option<ActionPreview> {
        self.detect(text).map(|action| ActionPreview {
            action_id: action.id.to_string(),
            kind: action.kind,
            reversible: action.reversible,
            summary: action.summary(),
        })
    }

    /// Whether a text asks for a destructive action.
    pub fn requires_confirmation(&self, text: &str) -> bool {
        self.detect(text).is_some()
    }

    /// The gate a caller must pass before acting.
    pub fn confirm(&self, text: &str) -> Confirmation {
        match self.preview(text) {
            Some(preview) => Confirmation::Required(preview),
            None => Confirmation::NotRequired,
        }
    }
}

/// A plain-language preview of what an action will do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionPreview {
    action_id: String,
    kind: DestructiveKind,
    reversible: bool,
    summary: String,
}

impl ActionPreview {
    /// The action id.
    pub fn action_id(&self) -> &str {
        &self.action_id
    }

    /// The consequence kind.
    pub fn kind(&self) -> DestructiveKind {
        self.kind
    }

    /// Whether the action can be undone.
    pub fn is_reversible(&self) -> bool {
        self.reversible
    }

    /// The sentence to show before acting.
    pub fn summary(&self) -> &str {
        &self.summary
    }
}

/// Whether a request may proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confirmation {
    /// A destructive action was detected; show the preview and ask.
    Required(ActionPreview),
    /// Nothing destructive was detected.
    NotRequired,
}

impl Confirmation {
    /// Whether the request is destructive.
    pub fn is_required(&self) -> bool {
        matches!(self, Self::Required(_))
    }

    /// The preview, when one is required.
    pub fn preview(&self) -> Option<&ActionPreview> {
        match self {
            Self::Required(preview) => Some(preview),
            Self::NotRequired => None,
        }
    }

    /// Whether the caller may proceed without asking.
    ///
    /// Only [`Confirmation::NotRequired`] may proceed. A `Required` preview must
    /// be shown and accepted by the user through another path; the catalog has
    /// no perform method that skips this.
    pub fn can_proceed(&self) -> bool {
        matches!(self, Self::NotRequired)
    }
}

/// Whether a lowercased text contains a trigger.
///
/// A single-word trigger sits on word boundaries. A multi-word trigger is a
/// phrase and matches as a substring, so "permanently delete" wins over
/// "delete" regardless of order.
fn contains_trigger(lower: &str, trigger: &str) -> bool {
    if trigger.contains(' ') {
        return lower.contains(trigger);
    }
    let bytes = lower.as_bytes();
    lower.match_indices(trigger).any(|(at, _)| {
        let before_ok = at == 0 || !is_word_byte(bytes[at - 1]);
        let end = at + trigger.len();
        let after_ok = end == bytes.len() || !is_word_byte(bytes[end]);
        before_ok && after_ok
    })
}

/// Whether a byte continues a word.
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The shipped action list.
fn builtin_actions() -> Vec<DestructiveAction> {
    vec![
        DestructiveAction::new(
            "delete",
            DestructiveKind::Delete,
            "delete the selected item",
            "It is removed from this device.",
            false,
            &["delete", "deletes", "deleting", "deleted"],
        ),
        DestructiveAction::new(
            "permanent-delete",
            DestructiveKind::Delete,
            "permanently delete the selected item",
            "It is gone and cannot be recovered.",
            false,
            &[
                "permanently delete",
                "permanently deleted",
                "permanently deletes",
                "empty trash",
                "empty the trash",
                "shred",
            ],
        ),
        DestructiveAction::new(
            "overwrite",
            DestructiveKind::Overwrite,
            "overwrite the file",
            "The previous contents are replaced.",
            false,
            &["overwrite", "overwrites", "overwriting"],
        ),
        DestructiveAction::new(
            "send",
            DestructiveKind::Send,
            "send the message",
            "It is delivered and cannot be unsent.",
            false,
            &["send", "sends", "sending", "submit"],
        ),
        DestructiveAction::new(
            "purchase",
            DestructiveKind::Purchase,
            "complete the purchase",
            "Your payment method is charged.",
            false,
            &["purchase", "buy", "checkout", "place order"],
        ),
        DestructiveAction::new(
            "cancel-subscription",
            DestructiveKind::Terminate,
            "cancel the subscription",
            "You lose access at the end of the billing period.",
            false,
            &[
                "unsubscribe",
                "cancel subscription",
                "cancel the subscription",
                "cancel my subscription",
                "cancel plan",
            ],
        ),
        DestructiveAction::new(
            "revoke",
            DestructiveKind::Revoke,
            "revoke the permission",
            "The app can no longer access it.",
            true,
            &["revoke", "revokes", "revoking"],
        ),
        DestructiveAction::new(
            "format",
            DestructiveKind::Format,
            "erase the disk",
            "Every file on it is destroyed.",
            false,
            &["format", "erase disk", "wipe disk"],
        ),
        DestructiveAction::new(
            "publish",
            DestructiveKind::Publish,
            "publish the post",
            "It becomes visible to everyone.",
            true,
            &["publish", "publishes", "publishing"],
        ),
        DestructiveAction::new(
            "transfer",
            DestructiveKind::Send,
            "transfer the money",
            "It leaves your account.",
            false,
            &["transfer", "send money", "wire transfer"],
        ),
        DestructiveAction::new(
            "end-session",
            DestructiveKind::Terminate,
            "end the session",
            "You are signed out.",
            true,
            &[
                "sign out",
                "sign me out",
                "log out",
                "end session",
                "terminate",
            ],
        ),
    ]
}

/// Requests that must produce a preview.
pub fn destructive_requests() -> Vec<&'static str> {
    vec![
        "Delete the selected file.",
        "Permanently delete the folder.",
        "Empty the trash.",
        "Overwrite the document.",
        "Send the message now.",
        "Complete the purchase.",
        "Cancel my subscription.",
        "Revoke camera access.",
        "Format the drive.",
        "Publish the draft.",
        "Transfer $200 to the supplier.",
        "Sign me out.",
    ]
}

/// Requests that are not destructive.
pub fn safe_requests() -> Vec<&'static str> {
    vec![
        "Open the document.",
        "Rename the file to report.",
        "Copy the text to the clipboard.",
        "Search for the setting.",
        "Show the preview of the slide.",
        "Add a new item to the list.",
        "Sort by name.",
        "Print the page.",
    ]
}

#[cfg(test)]
mod tests;
