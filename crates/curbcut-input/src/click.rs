//! Click-through routing and focus handoff.
//!
//! The overlay is the single decision point for a click: a click is either
//! consumed by the overlay or handed off to the real app, never both. See
//! [`../../../docs/CLICKS.md`](../../../docs/CLICKS.md).
//!
//! Three rules hold:
//!
//! - **Handoff is never accidental.** Only a click that lands on the app target
//!   ([`ClickTarget::App`]) can be handed off. A click on the overlay is always
//!   consumed.
//! - **Focus returns predictably.** A handoff owes focus. [`ClickRouter::focus_due`]
//!   reports the debt and [`ClickRouter::return_focus`] clears it.
//! - **No double-click.** A handoff is a single delivery. A second handoff inside
//!   `double_click_ms` is suppressed, so the app never receives two clicks for
//!   one gesture.

/// Where a click landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClickTarget {
    /// On the overlay surface.
    Overlay,
    /// On the real app, through a region the overlay does not cover.
    App,
}

/// What the router decided for a click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickDecision {
    /// The overlay handles the click.
    Consume,
    /// The click is handed off to the real app.
    Handoff,
}

/// Proof that a handoff happened and focus is owed.
///
/// Tokens are issued by [`ClickRouter`]; the id is a counter, not a coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HandoffToken {
    id: u64,
}

impl HandoffToken {
    fn new(id: u64) -> Self {
        Self { id }
    }

    /// The token's numeric id.
    pub fn get(self) -> u64 {
        self.id
    }
}

/// Routes clicks and tracks the focus handoff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClickRouter {
    double_click_ms: u64,
    last_handoff_ms: Option<u64>,
    focus_owed: Option<HandoffToken>,
    next_id: u64,
    consumes: u64,
    handoffs: u64,
    suppressed: u64,
}

impl ClickRouter {
    /// Creates a router.
    ///
    /// `double_click_ms` is the window inside which a second handoff is
    /// suppressed. A window of zero disables the guard.
    pub fn new(double_click_ms: u64) -> Self {
        Self {
            double_click_ms,
            last_handoff_ms: None,
            focus_owed: None,
            next_id: 0,
            consumes: 0,
            handoffs: 0,
            suppressed: 0,
        }
    }

    /// The configured double-click window, in milliseconds.
    pub fn double_click_ms(&self) -> u64 {
        self.double_click_ms
    }

    /// Routes one click observed at `at_ms`.
    ///
    /// A click on [`ClickTarget::Overlay`] is always [`ClickDecision::Consume`].
    /// A click on [`ClickTarget::App`] is [`ClickDecision::Handoff`] unless a
    /// handoff already happened inside the double-click window, in which case it
    /// is consumed and counted as suppressed.
    pub fn route(&mut self, target: ClickTarget, at_ms: u64) -> ClickDecision {
        match target {
            ClickTarget::Overlay => {
                self.consumes += 1;
                ClickDecision::Consume
            }
            ClickTarget::App => {
                let within_window = self
                    .last_handoff_ms
                    .is_some_and(|last| at_ms.saturating_sub(last) < self.double_click_ms);
                if within_window {
                    self.suppressed += 1;
                    return ClickDecision::Consume;
                }
                self.handoffs += 1;
                self.last_handoff_ms = Some(at_ms);
                let token = HandoffToken::new(self.next_id);
                self.next_id += 1;
                self.focus_owed = Some(token);
                ClickDecision::Handoff
            }
        }
    }

    /// The token for the handoff whose focus has not returned, if any.
    pub fn focus_due(&self) -> Option<&HandoffToken> {
        self.focus_owed.as_ref()
    }

    /// Returns focus to the overlay and clears the debt.
    ///
    /// Returns the token of the handoff that was outstanding, or `None` when
    /// focus was already in the overlay.
    pub fn return_focus(&mut self) -> Option<HandoffToken> {
        self.focus_owed.take()
    }

    /// How many clicks the overlay consumed.
    pub fn consume_count(&self) -> u64 {
        self.consumes
    }

    /// How many clicks were handed off.
    pub fn handoff_count(&self) -> u64 {
        self.handoffs
    }

    /// How many handoffs the double-click guard suppressed.
    pub fn suppressed_count(&self) -> u64 {
        self.suppressed
    }

    /// Forgets every click, token, and counter.
    pub fn reset(&mut self) {
        self.last_handoff_ms = None;
        self.focus_owed = None;
        self.next_id = 0;
        self.consumes = 0;
        self.handoffs = 0;
        self.suppressed = 0;
    }
}

#[cfg(test)]
mod tests;
