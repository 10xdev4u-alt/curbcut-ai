//! Audio focus.
//!
//! Speaking is a shared resource. An engine asks for focus, the app grants it,
//! and the engine gives it back. The state machine keeps those transitions
//! explicit so a second speaker cannot interrupt silently. See
//! [`../../../docs/SPEECH.md`](../../../docs/SPEECH.md).
//!
//! Three rules hold:
//!
//! - **Focus is requested before it is granted.** [`AudioFocus::grant`] from
//!   [`FocusState::Idle`] is ignored.
//! - **Only one holder.** A grant while already [`FocusState::Granted`] leaves
//!   the state unchanged.
//! - **Abandoning returns to idle.** [`AudioFocus::abandon`] from
//!   [`FocusState::Granted`] returns to [`FocusState::Idle`], so the next
//!   speaker starts clean.

/// The audio focus state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum FocusState {
    /// Nobody holds focus.
    #[default]
    Idle,
    /// Asked for, not yet granted.
    Requested,
    /// Held.
    Granted,
    /// A request was cancelled before it was granted.
    Abandoned,
}

/// The audio focus holder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AudioFocus {
    state: FocusState,
}

impl AudioFocus {
    /// Creates an idle focus.
    pub fn new() -> Self {
        Self::default()
    }

    /// The current state.
    pub fn state(&self) -> FocusState {
        self.state
    }

    /// Whether focus is held.
    pub fn is_granted(&self) -> bool {
        self.state == FocusState::Granted
    }

    /// Asks for focus.
    pub fn request(&mut self) -> FocusState {
        self.state = match self.state {
            FocusState::Idle | FocusState::Abandoned => FocusState::Requested,
            other => other,
        };
        self.state
    }

    /// Grants a request. A grant without a request is ignored.
    pub fn grant(&mut self) -> FocusState {
        self.state = match self.state {
            FocusState::Requested => FocusState::Granted,
            other => other,
        };
        self.state
    }

    /// Gives focus back.
    pub fn abandon(&mut self) -> FocusState {
        self.state = match self.state {
            FocusState::Granted => FocusState::Idle,
            FocusState::Requested => FocusState::Abandoned,
            other => other,
        };
        self.state
    }

    /// Resets to idle.
    pub fn reset(&mut self) {
        self.state = FocusState::Idle;
    }
}

#[cfg(test)]
mod tests;
