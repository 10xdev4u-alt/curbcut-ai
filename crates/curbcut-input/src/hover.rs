//! Global pointer tracking for hover-to-read and the lens.
//!
//! The tracker is platform-agnostic: it consumes pointer samples and decides
//! whether the pointer is moving or has settled over a target. The platform
//! adapter supplies the samples. See
//! [`../../../docs/HOVER.md`](../../../docs/HOVER.md).
//!
//! Three rules hold:
//!
//! - **A hover is dwell, not presence.** The pointer must stay within
//!   `move_threshold` of the dwell reference for `settle_ms` before the state is
//!   [`PointerState::Settled`].
//! - **A move is per-axis and measured from the dwell reference.** A sample
//!   counts as movement when either axis moves more than `move_threshold` from
//!   the reference, not from the previous sample. A slow drift that stays inside
//!   the box on every adjacent pair still counts as a move once it carries the
//!   pointer past the threshold.
//! - **Only positions and counts leave the tracker.** The tracker stores the
//!   last sample and counters. It never stores content, and the logging rule is
//!   stated in the docs.

use curbcut_core::types::{Point, Rect};

/// A pointer sample: a position and the time it was observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointerSample {
    position: Point,
    at_ms: u64,
}

impl PointerSample {
    /// Creates a sample.
    pub fn new(position: Point, at_ms: u64) -> Self {
        Self { position, at_ms }
    }

    /// The position.
    pub fn position(&self) -> Point {
        self.position
    }

    /// The time the sample was observed, in milliseconds.
    pub fn at_ms(&self) -> u64 {
        self.at_ms
    }
}

/// The pointer's movement state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerState {
    /// Fewer than two samples: movement is not yet known.
    Unknown,
    /// The pointer moved inside the settle window.
    Moving,
    /// The pointer stayed inside the threshold for the settle window.
    Settled,
}

impl PointerState {
    /// Whether the pointer is settled.
    pub fn is_settled(&self) -> bool {
        matches!(self, Self::Settled)
    }

    /// Whether the pointer is moving.
    pub fn is_moving(&self) -> bool {
        matches!(self, Self::Moving)
    }

    /// Whether the state is not yet known.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

/// A pointer tracker that classifies movement from samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverTracker {
    settle_ms: u64,
    move_threshold: i32,
    last: Option<PointerSample>,
    /// The position the dwell window is measured from. It advances only on a
    /// move, so a slow drift is still caught.
    dwell_reference: Option<Point>,
    last_move_ms: u64,
    dwell_ms: u64,
    samples: u64,
    moves: u64,
    path_length: u64,
}

impl HoverTracker {
    /// Creates a tracker.
    ///
    /// `settle_ms` is how long the pointer must stay inside `move_threshold`
    /// before it counts as settled. `move_threshold` is the per-axis distance in
    /// pixels that counts as a move; a negative threshold is treated as zero.
    pub fn new(settle_ms: u64, move_threshold: i32) -> Self {
        Self {
            settle_ms,
            move_threshold: move_threshold.max(0),
            last: None,
            dwell_reference: None,
            last_move_ms: 0,
            dwell_ms: 0,
            samples: 0,
            moves: 0,
            path_length: 0,
        }
    }

    /// The configured settle window, in milliseconds.
    pub fn settle_ms(&self) -> u64 {
        self.settle_ms
    }

    /// The configured per-axis move threshold, in pixels.
    pub fn move_threshold(&self) -> i32 {
        self.move_threshold
    }

    /// Feeds a sample.
    ///
    /// The first sample establishes the position and is always [`PointerState::Unknown`].
    /// Later samples update the dwell time and the movement counters.
    ///
    /// Movement is measured from the dwell reference, not from the previous
    /// sample, so a slow drift cannot accumulate dwell. Path length still uses
    /// the adjacent-sample delta. A timestamp earlier than the previous sample
    /// restarts the dwell window instead of reporting a large dwell.
    pub fn update(&mut self, sample: PointerSample) {
        self.samples += 1;
        if let Some(previous) = self.last {
            let dx = i64::from(sample.position.x) - i64::from(previous.position.x);
            let dy = i64::from(sample.position.y) - i64::from(previous.position.y);
            self.path_length = self
                .path_length
                .saturating_add((dx.abs() + dy.abs()) as u64);

            let reference = self.dwell_reference.unwrap_or(previous.position);
            let rx = i64::from(sample.position.x) - i64::from(reference.x);
            let ry = i64::from(sample.position.y) - i64::from(reference.y);
            let threshold = i64::from(self.move_threshold);
            let moved = rx.abs() > threshold || ry.abs() > threshold;
            if moved {
                self.moves += 1;
                self.dwell_reference = Some(sample.position);
                self.last_move_ms = sample.at_ms;
                self.dwell_ms = 0;
            } else {
                if sample.at_ms < previous.at_ms {
                    // The clock went backwards: restart the dwell window at the
                    // previous sample rather than reporting a stale large dwell.
                    self.last_move_ms = previous.at_ms;
                }
                self.dwell_ms = sample.at_ms.saturating_sub(self.last_move_ms);
            }
        } else {
            self.dwell_reference = Some(sample.position);
            self.last_move_ms = sample.at_ms;
            self.dwell_ms = 0;
        }
        self.last = Some(sample);
    }

    /// The most recent position, if any.
    pub fn position(&self) -> Option<Point> {
        self.last.map(|sample| sample.position())
    }

    /// The current movement state.
    pub fn state(&self) -> PointerState {
        if self.samples < 2 {
            return PointerState::Unknown;
        }
        if self.dwell_ms >= self.settle_ms {
            PointerState::Settled
        } else {
            PointerState::Moving
        }
    }

    /// How long the pointer has been inside the move threshold, in milliseconds.
    pub fn dwell_ms(&self) -> u64 {
        self.dwell_ms
    }

    /// Whether the most recent position is inside `rect`.
    ///
    /// The rectangle is half-open, matching [`Rect::contains`]: the left and
    /// top edges are inside, the right and bottom edges are outside.
    pub fn is_over(&self, rect: Rect) -> bool {
        self.last
            .is_some_and(|sample| rect.contains(sample.position))
    }

    /// How many samples were fed.
    pub fn sample_count(&self) -> u64 {
        self.samples
    }

    /// How many samples counted as movement.
    pub fn move_count(&self) -> u64 {
        self.moves
    }

    /// Total Manhattan distance travelled, in pixels.
    pub fn path_length_px(&self) -> u64 {
        self.path_length
    }

    /// Forgets every sample and counter.
    pub fn reset(&mut self) {
        self.last = None;
        self.dwell_reference = None;
        self.last_move_ms = 0;
        self.dwell_ms = 0;
        self.samples = 0;
        self.moves = 0;
        self.path_length = 0;
    }
}

#[cfg(test)]
mod tests;
