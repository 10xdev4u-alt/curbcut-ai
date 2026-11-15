//! The global shortcut catalog.
//!
//! Every global action that can have a shortcut is listed in
//! [`ShortcutAction`]. [`ShortcutCatalog::defaults`] is the ergonomic default
//! set, [`ShortcutCatalog::rebind`] changes one binding, and
//! [`ShortcutCatalog::conflicts`] reports any two actions that share a chord.
//!
//! Three rules hold:
//!
//! - **Defaults do not conflict.** The default catalog has no two actions on
//!   the same chord, and every default chord is valid.
//! - **A chord is a set, not a sequence.** `Ctrl+Shift+D` and
//!   `Shift+Ctrl+D` are the same chord, so lookup is order-independent and
//!   case-insensitive.
//! - **A taken chord is rejected.** `rebind` refuses a chord another action
//!   already uses, so a conflict can only exist if it is introduced directly.
//!
//! The platform layer maps a pressed chord to a [`ShortcutAction`] with
//! [`ShortcutCatalog::action_for`] and never has to know the defaults.

use curbcut_core::config::Shortcut;
use curbcut_core::error::CurbCutError;

/// A global action that can have a shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShortcutAction {
    /// Hide the overlay, stop capture, mute audio, restore the app.
    Panic,
    /// Toggle dimming.
    ToggleDim,
    /// Toggle the icon grid.
    ToggleGrid,
    /// Toggle reduced motion.
    ToggleMotion,
    /// Toggle the reading ruler.
    ToggleRuler,
    /// Toggle read-aloud.
    ToggleReadAloud,
    /// Read from the current focus.
    ReadFromHere,
    /// Move to the next tile.
    NextTile,
    /// Move to the previous tile.
    PreviousTile,
    /// Activate the focused tile.
    Activate,
    /// Open the profile picker.
    Profiles,
    /// Open the interruption inbox.
    Inbox,
}

impl ShortcutAction {
    /// Every action, in catalog order.
    pub const ALL: &'static [ShortcutAction] = &[
        Self::Panic,
        Self::ToggleDim,
        Self::ToggleGrid,
        Self::ToggleMotion,
        Self::ToggleRuler,
        Self::ToggleReadAloud,
        Self::ReadFromHere,
        Self::NextTile,
        Self::PreviousTile,
        Self::Activate,
        Self::Profiles,
        Self::Inbox,
    ];

    /// The stable string form used in configuration and the UI.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Panic => "panic",
            Self::ToggleDim => "toggle_dim",
            Self::ToggleGrid => "toggle_grid",
            Self::ToggleMotion => "toggle_motion",
            Self::ToggleRuler => "toggle_ruler",
            Self::ToggleReadAloud => "toggle_read_aloud",
            Self::ReadFromHere => "read_from_here",
            Self::NextTile => "next_tile",
            Self::PreviousTile => "previous_tile",
            Self::Activate => "activate",
            Self::Profiles => "profiles",
            Self::Inbox => "inbox",
        }
    }

    /// The default chord for this action.
    pub fn default_keys(self) -> Shortcut {
        match self {
            Self::Panic => chord(&["ctrl", "shift", "esc"]),
            Self::ToggleDim => chord(&["ctrl", "shift", "d"]),
            Self::ToggleGrid => chord(&["ctrl", "shift", "g"]),
            Self::ToggleMotion => chord(&["ctrl", "shift", "m"]),
            Self::ToggleRuler => chord(&["ctrl", "shift", "r"]),
            Self::ToggleReadAloud => chord(&["ctrl", "shift", "a"]),
            Self::ReadFromHere => chord(&["ctrl", "shift", "enter"]),
            Self::NextTile => chord(&["ctrl", "shift", "right"]),
            Self::PreviousTile => chord(&["ctrl", "shift", "left"]),
            Self::Activate => chord(&["ctrl", "shift", "space"]),
            Self::Profiles => chord(&["ctrl", "shift", "p"]),
            Self::Inbox => chord(&["ctrl", "shift", "i"]),
        }
    }

    /// One sentence describing what the action does.
    pub fn doc(self) -> &'static str {
        match self {
            Self::Panic => "Hide the overlay, stop capture, mute audio, and restore the app.",
            Self::ToggleDim => "Turn dimming on or off.",
            Self::ToggleGrid => "Turn the icon grid on or off.",
            Self::ToggleMotion => "Turn reduced motion on or off.",
            Self::ToggleRuler => "Turn the reading ruler on or off.",
            Self::ToggleReadAloud => "Turn read-aloud on or off.",
            Self::ReadFromHere => "Read the focused text from here.",
            Self::NextTile => "Move to the next tile in the grid.",
            Self::PreviousTile => "Move to the previous tile in the grid.",
            Self::Activate => "Activate the focused tile.",
            Self::Profiles => "Open the profile picker.",
            Self::Inbox => "Open the interruption inbox.",
        }
    }
}

/// Why a rebind was rejected.
#[derive(Debug, Clone, PartialEq)]
pub enum ShortcutError {
    /// The chord is empty or names an unknown key.
    Invalid(String),
    /// Another action already uses the chord.
    Conflict {
        /// The action that already uses it.
        other: ShortcutAction,
        /// The chord that was requested.
        keys: Shortcut,
    },
}

impl std::fmt::Display for ShortcutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => write!(f, "invalid shortcut: {message}"),
            Self::Conflict { other, keys } => write!(
                f,
                "{} is already bound to {}",
                format_keys(keys),
                other.as_str()
            ),
        }
    }
}

impl std::error::Error for ShortcutError {}

/// The global shortcut bindings.
#[derive(Debug, Clone, PartialEq)]
pub struct ShortcutCatalog {
    bindings: Vec<(ShortcutAction, Shortcut)>,
}

impl Default for ShortcutCatalog {
    fn default() -> Self {
        Self::defaults()
    }
}

impl ShortcutCatalog {
    /// The ergonomic default catalog.
    pub fn defaults() -> Self {
        Self {
            bindings: ShortcutAction::ALL
                .iter()
                .map(|action| (*action, action.default_keys()))
                .collect(),
        }
    }

    /// How many actions are bound.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Whether nothing is bound.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Every bound action, in catalog order.
    pub fn actions(&self) -> impl Iterator<Item = ShortcutAction> + '_ {
        self.bindings.iter().map(|(action, _)| *action)
    }

    /// The chord bound to an action.
    pub fn binding(&self, action: ShortcutAction) -> Option<&Shortcut> {
        self.bindings
            .iter()
            .find(|(candidate, _)| *candidate == action)
            .map(|(_, keys)| keys)
    }

    /// The action a pressed chord triggers, if any.
    ///
    /// The chord is compared as a set, so key order and letter case do not
    /// matter. If a catalog has two actions on the same chord, the first in
    /// catalog order wins; `conflicts` reports the pair, and `rebind` prevents
    /// it through the public API.
    pub fn action_for(&self, keys: &Shortcut) -> Option<ShortcutAction> {
        let pressed = normalize(keys);
        self.bindings
            .iter()
            .find(|(_, candidate)| normalize(candidate) == pressed)
            .map(|(action, _)| *action)
    }

    /// Rebinds an action to a new chord.
    ///
    /// The chord must be non-empty, name only known keys, and not already be
    /// used by another action.
    ///
    /// # Errors
    ///
    /// Returns [`ShortcutError::Invalid`] for an empty or unknown key, and
    /// [`ShortcutError::Conflict`] when another action already uses the chord.
    pub fn rebind(&mut self, action: ShortcutAction, keys: Shortcut) -> Result<(), ShortcutError> {
        keys.validate("shortcut").map_err(|error| match error {
            // `Config` already carries the field path; keep the message and
            // drop the `"invalid configuration: "` prefix so `Display` does
            // not prepend a second one.
            CurbCutError::Config { message } => ShortcutError::Invalid(message),
            other => ShortcutError::Invalid(other.to_string()),
        })?;
        let requested = normalize(&keys);
        if let Some((other, _)) = self
            .bindings
            .iter()
            .find(|(candidate, bound)| *candidate != action && normalize(bound) == requested)
        {
            return Err(ShortcutError::Conflict {
                other: *other,
                keys: keys.clone(),
            });
        }
        let entry = self
            .bindings
            .iter_mut()
            .find(|(candidate, _)| *candidate == action)
            .ok_or_else(|| ShortcutError::Invalid(format!("unknown action {}", action.as_str())))?;
        entry.1 = keys;
        Ok(())
    }

    /// Every pair of actions that share a chord.
    ///
    /// The default catalog has none. A pair is reported once, in catalog order.
    pub fn conflicts(&self) -> Vec<(ShortcutAction, ShortcutAction, Shortcut)> {
        let mut conflicts = Vec::new();
        for (index, (left_action, left_keys)) in self.bindings.iter().enumerate() {
            for (right_action, right_keys) in &self.bindings[index + 1..] {
                if normalize(left_keys) == normalize(right_keys) {
                    conflicts.push((*left_action, *right_action, left_keys.clone()));
                }
            }
        }
        conflicts
    }

    /// Whether an action still has its default chord.
    pub fn is_default(&self, action: ShortcutAction) -> bool {
        self.binding(action)
            .is_some_and(|keys| normalize(keys) == normalize(&action.default_keys()))
    }
}

/// Builds a chord from key names.
fn chord(keys: &[&str]) -> Shortcut {
    Shortcut {
        keys: keys.iter().map(|key| key.to_string()).collect(),
    }
}

/// Lowercases, sorts, and deduplicates key names.
fn normalize(keys: &Shortcut) -> Vec<String> {
    let mut normalized: Vec<String> = keys.keys.iter().map(|key| key.to_lowercase()).collect();
    normalized.sort_unstable();
    normalized.dedup();
    normalized
}

/// Formats a chord for display, in the order given.
fn format_keys(keys: &Shortcut) -> String {
    keys.keys.join("+")
}

#[cfg(test)]
mod tests;
