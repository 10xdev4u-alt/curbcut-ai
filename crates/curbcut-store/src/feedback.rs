//! Learned simplifications from accept/reject feedback.
//!
//! [`LearnedSimplification`](crate::feedback::LearnedSimplification) records
//! whether the user accepted or rejected a simplification, keyed by a content
//! hash, and moves a simplification level within bounds. See
//! [`../../../docs/LEARNED_SIMPLIFICATIONS.md`](../../../docs/LEARNED_SIMPLIFICATIONS.md).
//!
//! Four rules hold:
//!
//! - **Feedback is keyed by a hash, never by content.** A feedback entry holds
//!   a [`ContentHash`](crate::plan_cache::ContentHash), a
//!   [`Feedback`](crate::feedback::Feedback), and a sequence number. No text
//!   from the screen is stored or logged.
//! - **Adaptation is bounded.** The level moves by at most
//!   [`step`](crate::feedback::AdaptationConfig::step) per adaptation, never
//!   outside
//!   [`min_level`](crate::feedback::AdaptationConfig::min_level)..=
//!   [`max_level`](crate::feedback::AdaptationConfig::max_level), and only after
//!   [`min_observations`](crate::feedback::AdaptationConfig::min_observations)
//!   entries and
//!   [`min_records_between`](crate::feedback::AdaptationConfig::min_records_between)
//!   records since the last move.
//! - **Adaptation is reversible.**
//!   [`undo`](crate::feedback::LearnedSimplification::undo) restores the
//!   previous level; the history is bounded by
//!   [`history_limit`](crate::feedback::AdaptationConfig::history_limit).
//! - **The log is bounded.**
//!   [`FeedbackLog`](crate::feedback::FeedbackLog) keeps at most its capacity
//!   and evicts the oldest entry.
//!
//! The type is a pure data structure: no I/O, no clock, and no network. The
//! accept rate is reported; nothing leaves the device.

use std::collections::VecDeque;

use crate::plan_cache::ContentHash;

/// The default fewest entries before the level may move.
pub const DEFAULT_MIN_OBSERVATIONS: usize = 5;
/// The default fewest records between two moves.
pub const DEFAULT_MIN_RECORDS_BETWEEN: usize = 4;
/// The default bound on the undo history.
pub const DEFAULT_HISTORY_LIMIT: usize = 16;
/// The default log capacity.
pub const DEFAULT_CAPACITY: usize = 256;
/// The default lowest simplification level.
pub const DEFAULT_MIN_LEVEL: u8 = 1;
/// The default highest simplification level.
pub const DEFAULT_MAX_LEVEL: u8 = 20;
/// The default level change per adaptation.
pub const DEFAULT_STEP: u8 = 1;
/// The default accept rate at or above which the level rises.
pub const DEFAULT_HIGH: f32 = 0.8;
/// The default accept rate at or below which the level falls.
pub const DEFAULT_LOW: f32 = 0.4;

/// Whether the user accepted or rejected a simplification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feedback {
    /// The user kept the simplification.
    Accepted,
    /// The user reverted it.
    Rejected,
}

impl Feedback {
    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }

    /// Whether this is an acceptance.
    pub const fn is_accept(self) -> bool {
        matches!(self, Self::Accepted)
    }

    /// Whether this is a rejection.
    pub const fn is_reject(self) -> bool {
        matches!(self, Self::Rejected)
    }
}

/// One recorded feedback, keyed by a content hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedbackEntry {
    content: ContentHash,
    feedback: Feedback,
    sequence: u64,
}

impl FeedbackEntry {
    /// Creates an entry.
    pub const fn new(content: ContentHash, feedback: Feedback, sequence: u64) -> Self {
        Self {
            content,
            feedback,
            sequence,
        }
    }

    /// The content hash.
    pub const fn content(&self) -> ContentHash {
        self.content
    }

    /// The feedback.
    pub const fn feedback(&self) -> Feedback {
        self.feedback
    }

    /// The 1-based-free sequence number, starting at zero.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
}

/// A bounded log of feedback, oldest first.
#[derive(Debug, Clone)]
pub struct FeedbackLog {
    capacity: usize,
    entries: VecDeque<FeedbackEntry>,
    next_sequence: u64,
}

impl Default for FeedbackLog {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

impl FeedbackLog {
    /// Creates a log that holds at most `capacity` entries.
    ///
    /// A capacity of zero stores nothing.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: VecDeque::new(),
            next_sequence: 0,
        }
    }

    /// The maximum number of entries.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entries, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &FeedbackEntry> {
        self.entries.iter()
    }

    /// Records feedback, evicting the oldest entry when the log is full.
    ///
    /// Returns the stored entry, or `None` when the capacity is zero.
    pub fn record(&mut self, content: ContentHash, feedback: Feedback) -> Option<FeedbackEntry> {
        if self.capacity == 0 {
            return None;
        }
        let entry = FeedbackEntry::new(content, feedback, self.next_sequence);
        self.next_sequence += 1;
        self.entries.push_back(entry);
        while self.entries.len() > self.capacity {
            self.entries.pop_front();
        }
        Some(entry)
    }

    /// The number of acceptances.
    pub fn accepts(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.feedback.is_accept())
            .count()
    }

    /// The number of rejections.
    pub fn rejects(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.feedback.is_reject())
            .count()
    }

    /// The number of entries.
    pub fn total(&self) -> usize {
        self.entries.len()
    }

    /// The accept rate over the whole log, or `None` when it is empty.
    pub fn accept_rate(&self) -> Option<f32> {
        if self.entries.is_empty() {
            return None;
        }
        Some(self.accepts() as f32 / self.total() as f32)
    }

    /// The accept rate over the newest `n` entries, or `None` when `n` is zero
    /// or the log is empty.
    pub fn accept_rate_last(&self, n: usize) -> Option<f32> {
        if n == 0 || self.entries.is_empty() {
            return None;
        }
        let start = self.total().saturating_sub(n);
        let total = self.total() - start;
        let accepts = self
            .entries
            .iter()
            .skip(start)
            .filter(|entry| entry.feedback.is_accept())
            .count();
        Some(accepts as f32 / total as f32)
    }

    /// Drops every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// How the simplification level may move.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptationConfig {
    /// The lowest level.
    pub min_level: u8,
    /// The highest level.
    pub max_level: u8,
    /// The most the level moves per adaptation.
    pub step: u8,
    /// The accept rate at or above which the level rises.
    pub high: f32,
    /// The accept rate at or below which the level falls.
    pub low: f32,
    /// The fewest entries before the level may move.
    pub min_observations: usize,
    /// The fewest records between two moves.
    pub min_records_between: usize,
    /// The bound on the undo history.
    pub history_limit: usize,
}

impl Default for AdaptationConfig {
    fn default() -> Self {
        Self {
            min_level: DEFAULT_MIN_LEVEL,
            max_level: DEFAULT_MAX_LEVEL,
            step: DEFAULT_STEP,
            high: DEFAULT_HIGH,
            low: DEFAULT_LOW,
            min_observations: DEFAULT_MIN_OBSERVATIONS,
            min_records_between: DEFAULT_MIN_RECORDS_BETWEEN,
            history_limit: DEFAULT_HISTORY_LIMIT,
        }
    }
}

impl AdaptationConfig {
    /// Creates a configuration.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        min_level: u8,
        max_level: u8,
        step: u8,
        high: f32,
        low: f32,
        min_observations: usize,
        min_records_between: usize,
        history_limit: usize,
    ) -> Self {
        Self {
            min_level,
            max_level,
            step,
            high,
            low,
            min_observations,
            min_records_between,
            history_limit,
        }
    }

    /// Returns a configuration with every field in a valid range.
    ///
    /// A reversed level range is swapped, a reversed rate range is swapped, a
    /// zero step becomes one, rates are clamped to `0.0..=1.0`, and a zero
    /// history limit becomes one.
    pub fn clamp(self) -> Self {
        let (min_level, max_level) = if self.min_level <= self.max_level {
            (self.min_level, self.max_level)
        } else {
            (self.max_level, self.min_level)
        };
        let (low, high) = if self.low <= self.high {
            (self.low, self.high)
        } else {
            (self.high, self.low)
        };
        Self {
            min_level,
            max_level,
            step: self.step.max(1),
            high: high.clamp(0.0, 1.0),
            low: low.clamp(0.0, 1.0),
            min_observations: self.min_observations,
            min_records_between: self.min_records_between,
            history_limit: self.history_limit.max(1),
        }
    }
}

/// A simplification level that learns from accept/reject feedback.
#[derive(Debug, Clone)]
pub struct LearnedSimplification {
    log: FeedbackLog,
    config: AdaptationConfig,
    level: u8,
    history: Vec<u8>,
    records_since_adapt: usize,
}

impl LearnedSimplification {
    /// Creates a learner that starts at the lowest level.
    pub fn new(capacity: usize, config: AdaptationConfig) -> Self {
        let config = config.clamp();
        Self {
            log: FeedbackLog::new(capacity),
            level: config.min_level,
            config,
            history: Vec::new(),
            records_since_adapt: 0,
        }
    }

    /// The configuration.
    pub const fn config(&self) -> &AdaptationConfig {
        &self.config
    }

    /// The current simplification level.
    pub const fn level(&self) -> u8 {
        self.level
    }

    /// The feedback log.
    pub const fn log(&self) -> &FeedbackLog {
        &self.log
    }

    /// The number of recorded entries.
    pub fn len(&self) -> usize {
        self.log.len()
    }

    /// Whether no feedback is recorded.
    pub fn is_empty(&self) -> bool {
        self.log.is_empty()
    }

    /// The accept rate over the whole log.
    pub fn accept_rate(&self) -> Option<f32> {
        self.log.accept_rate()
    }

    /// The accept rate over the newest `n` entries.
    pub fn accept_rate_last(&self, n: usize) -> Option<f32> {
        self.log.accept_rate_last(n)
    }

    /// Whether [`Self::undo`] would change the level.
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    /// Records feedback and adapts the level.
    ///
    /// Returns whether the level moved. The level moves only once enough
    /// evidence has accumulated and only after the rate limiter allows it.
    pub fn record(&mut self, content: ContentHash, feedback: Feedback) -> bool {
        if self.log.record(content, feedback).is_none() {
            return false;
        }
        self.records_since_adapt += 1;
        if self.log.len() < self.config.min_observations
            || self.records_since_adapt < self.config.min_records_between
        {
            return false;
        }
        let Some(rate) = self.log.accept_rate() else {
            return false;
        };
        let next = if rate >= self.config.high {
            self.level
                .saturating_add(self.config.step)
                .min(self.config.max_level)
        } else if rate <= self.config.low {
            self.level
                .saturating_sub(self.config.step)
                .max(self.config.min_level)
        } else {
            return false;
        };
        if next == self.level {
            return false;
        }
        self.history.push(self.level);
        let overflow = self.history.len().saturating_sub(self.config.history_limit);
        if overflow > 0 {
            self.history.drain(0..overflow);
        }
        self.level = next;
        self.records_since_adapt = 0;
        true
    }

    /// Restores the previous level. Returns whether it moved.
    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.history.pop() else {
            return false;
        };
        self.level = previous;
        self.records_since_adapt = 0;
        true
    }

    /// Drops every feedback entry, every undo step, and resets the level.
    pub fn clear(&mut self) {
        self.log.clear();
        self.history.clear();
        self.level = self.config.min_level;
        self.records_since_adapt = 0;
    }
}

#[cfg(test)]
mod tests;
