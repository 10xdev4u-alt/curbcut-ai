//! A local, content-free crash-report artifact and its approval gate.
//!
//! This module builds a string in process and hands it back to the caller. It
//! has no uploader, no socket, no recipient, no approval UI, and no channel:
//! nothing here leaves the device, and #219 remains open. F-08 and DPIA R-07
//! remain open too. The artifact holds counts and fixed codes only, so no
//! screen text, OCR text, or accessibility string can be stored in it.
//!
//! [`CrashReportGate`] is a **local decision primitive**: approval decides
//! whether [`CrashReportGate::prepare`] produces an artifact at all, and that
//! approval is consumed by the one call that succeeds. It is not a sender.
//! See [`../../../docs/TRANSMISSION_PATHS.md`](../../../docs/TRANSMISSION_PATHS.md).

use std::fmt;

/// Why a local crash summary was produced.
///
/// Each variant renders as a fixed code and carries no text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrashReason {
    /// An unhandled panic was observed locally.
    Panic,
    /// A required permission was revoked.
    PermissionRevoked,
    /// A local cache was found to be corrupt.
    CacheCorrupt,
}

impl CrashReason {
    /// Every reason, in render order.
    pub const ALL: [Self; 3] = [Self::Panic, Self::PermissionRevoked, Self::CacheCorrupt];

    /// The fixed code used in a rendered report.
    ///
    /// The returned string is a static label, never user or screen content.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Panic => "panic",
            Self::PermissionRevoked => "permission_revoked",
            Self::CacheCorrupt => "cache_corrupt",
        }
    }
}

/// A content-free summary of a local crash.
///
/// Numbers and a reason code are the only fields. There is no string field in
/// which text could be smuggled into a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CrashSummary {
    events: u64,
    reason: CrashReason,
    digest: u64,
}

impl CrashSummary {
    /// Builds a summary from counted events, a reason code, and a digest.
    pub const fn new(events: u64, reason: CrashReason, digest: u64) -> Self {
        Self {
            events,
            reason,
            digest,
        }
    }

    /// The number of locally counted events.
    pub const fn events(&self) -> u64 {
        self.events
    }

    /// The reason code for this summary.
    pub const fn reason(&self) -> CrashReason {
        self.reason
    }

    /// The content-free digest for this summary.
    pub const fn digest(&self) -> u64 {
        self.digest
    }
}

/// A rendered, content-free crash artifact.
///
/// Created by [`CrashReportGate::prepare`] alone. It stays in process: there
/// is no sender, recipient, or channel attached to it.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CrashReport {
    events: u64,
    reason: CrashReason,
    digest: u64,
}

impl CrashReport {
    /// Renders exactly `events=… reason=… digest=…`.
    ///
    /// Every value is a count, a fixed reason code, or a digest. No field can
    /// carry screen text, OCR text, or an accessibility string.
    pub fn render(&self) -> String {
        format!(
            "events={} reason={} digest={}",
            self.events,
            self.reason.as_str(),
            self.digest
        )
    }
}

impl fmt::Debug for CrashReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CrashReport")
            .field("events", &self.events)
            .field("reason", &self.reason.as_str())
            .field("digest", &self.digest)
            .finish()
    }
}

/// A local, one-shot approval decision for crash reports.
///
/// A new gate is denied. [`CrashReportGate::approve`] arms exactly one
/// preparation, and [`CrashReportGate::prepare`] consumes that approval when
/// it succeeds; [`CrashReportGate::revoke`] cancels it before then. This is a
/// decision in memory, not a user control: approving does not send anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrashReportGate {
    approved: bool,
}

impl CrashReportGate {
    /// A gate in the denied state.
    pub const fn new() -> Self {
        Self { approved: false }
    }

    /// Arms one preparation. Calling it again does not stack approvals.
    pub fn approve(&mut self) {
        self.approved = true;
    }

    /// Cancels a pending approval.
    pub fn revoke(&mut self) {
        self.approved = false;
    }

    /// Whether one preparation is currently approved.
    pub const fn is_approved(&self) -> bool {
        self.approved
    }

    /// Builds a local artifact if and only if this report was approved.
    ///
    /// Returns [`None`] while denied. A successful call consumes the approval,
    /// so a second call is denied until [`Self::approve`] is called again.
    /// The returned artifact stays in process; this function never dials.
    pub fn prepare(&mut self, summary: CrashSummary) -> Option<CrashReport> {
        if !self.approved {
            return None;
        }
        self.approved = false;
        Some(CrashReport {
            events: summary.events,
            reason: summary.reason,
            digest: summary.digest,
        })
    }
}

impl Default for CrashReportGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
