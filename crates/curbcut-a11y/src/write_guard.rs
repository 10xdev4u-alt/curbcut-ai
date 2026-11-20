//! Write guard for accessibility actions.
//!
//! Nothing in this crate writes to another application unless the caller holds
//! a valid [`CapabilityToken`] for [`Capability::WriteA11y`]. Every attempt is
//! recorded, allowed or denied, and a denied attempt never reaches the sink.
//! See [`../../../docs/WRITE_GUARD.md`](../../../docs/WRITE_GUARD.md).

use std::fmt;

use curbcut_core::capability::{require, Capability, CapabilityToken};
use curbcut_core::id::ElementId;

/// A write the adapter may attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriteAction {
    /// Set an accessibility value.
    SetValue,
    /// Invoke the element's default action.
    Invoke,
    /// Remove the element.
    Remove,
}

impl WriteAction {
    /// The stable string form used in logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SetValue => "set_value",
            Self::Invoke => "invoke",
            Self::Remove => "remove",
        }
    }
}

impl fmt::Display for WriteAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether an attempted write reached the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    /// The token was valid and the sink ran.
    Allowed,
    /// The token lacked the capability or was no longer valid.
    Denied,
}

impl WriteOutcome {
    /// The stable string form used in logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Denied => "denied",
        }
    }
}

impl fmt::Display for WriteOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One attempted write.
///
/// It carries a sequence number, an element id, an action kind, and an outcome.
/// It never carries screen content or raw accessibility strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteRecord {
    /// Position in the attempt sequence, starting at 1.
    pub sequence: u64,
    /// The target element.
    pub target: ElementId,
    /// The attempted action.
    pub action: WriteAction,
    /// Whether it reached the application.
    pub outcome: WriteOutcome,
}

/// The effect an allowed write has. Platform adapters implement this.
pub trait WriteSink {
    /// Performs the write.
    fn write(&mut self, target: ElementId, action: WriteAction);
}

/// A sink that discards every write; the default.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopSink;

impl WriteSink for NoopSink {
    fn write(&mut self, _target: ElementId, _action: WriteAction) {}
}

/// Guards every write attempt behind a capability check.
///
/// [`WriteGuard::request`] is the only write path: it requires a
/// [`CapabilityToken`], checks it with [`require`], records the attempt, and
/// calls the sink only when the token proves [`Capability::WriteA11y`].
pub struct WriteGuard {
    sink: Box<dyn WriteSink>,
    log: Vec<WriteRecord>,
    next_sequence: u64,
}

impl WriteGuard {
    /// Creates a guard with a [`NoopSink`].
    pub fn new() -> Self {
        Self::with_sink(NoopSink)
    }

    /// Creates a guard that writes through `sink`.
    pub fn with_sink(sink: impl WriteSink + 'static) -> Self {
        Self {
            sink: Box::new(sink),
            log: Vec::new(),
            next_sequence: 1,
        }
    }

    /// Attempts one write.
    ///
    /// A token that is revoked, or that proves a different capability, is
    /// denied: the sink is not called and the attempt is still recorded.
    pub fn request(
        &mut self,
        token: &CapabilityToken,
        target: ElementId,
        action: WriteAction,
    ) -> WriteOutcome {
        let outcome = if require(token, Capability::WriteA11y).is_ok() {
            self.sink.write(target, action);
            WriteOutcome::Allowed
        } else {
            WriteOutcome::Denied
        };
        let record = WriteRecord {
            sequence: self.next_sequence,
            target,
            action,
            outcome,
        };
        self.next_sequence += 1;
        self.log.push(record);
        outcome
    }

    /// Every attempt, oldest first.
    pub fn log(&self) -> &[WriteRecord] {
        &self.log
    }

    /// The number of attempts.
    pub fn len(&self) -> usize {
        self.log.len()
    }

    /// Whether no attempt has been recorded.
    pub fn is_empty(&self) -> bool {
        self.log.is_empty()
    }

    /// The number of attempts that reached the application.
    pub fn allowed(&self) -> usize {
        self.log
            .iter()
            .filter(|record| record.outcome == WriteOutcome::Allowed)
            .count()
    }

    /// The number of attempts that were denied.
    pub fn denied(&self) -> usize {
        self.log
            .iter()
            .filter(|record| record.outcome == WriteOutcome::Denied)
            .count()
    }
}

impl Default for WriteGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
