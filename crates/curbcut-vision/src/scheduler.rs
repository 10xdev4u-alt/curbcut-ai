//! Motion-aware frame scheduling.
//!
//! The scheduler decides which frames are worth a model call. It never sees
//! pixels: it compares [`MotionSignature`]s, which are fixed-size samples.
//!
//! Rules:
//!
//! - A protected frame is never analyzed and never changes scheduler state.
//! - A frame whose motion against the previous frame is below the threshold is
//!   skipped.
//! - A moving frame inside the coalescing window is skipped, so a burst of
//!   motion produces one analysis instead of one per frame.
//! - The first frame after a quiet period is analyzed.
//!
//! The scheduler is deterministic: it reads `FrameMeta::captured_at`, never the
//! wall clock, and it holds no interior mutability.

use std::fmt;
use std::time::Duration;

use crate::frame::VisionFrame;
use crate::motion::MotionSignature;

/// The default motion threshold: 8% of the sampled bytes changed.
pub const DEFAULT_MOTION_THRESHOLD: f32 = 0.08;

/// The default coalescing window.
pub const DEFAULT_COALESCE: Duration = Duration::from_millis(500);

/// Why a frame was not analyzed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// The frame is protected and must never reach a model.
    Protected,
    /// Motion against the previous frame is below the threshold.
    Static,
    /// Motion is above the threshold, but a recent frame was already analyzed.
    Coalesced,
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protected => write!(f, "protected"),
            Self::Static => write!(f, "static"),
            Self::Coalesced => write!(f, "coalesced"),
        }
    }
}

/// Content-free counters for a scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SchedulerStats {
    /// Frames offered to the scheduler.
    pub offered: u64,
    /// Frames sent to the models.
    pub analyzed: u64,
    /// Frames skipped because they were protected.
    pub protected: u64,
    /// Frames skipped because they were static.
    pub r#static: u64,
    /// Frames skipped because they were coalesced.
    pub coalesced: u64,
}

/// Decides whether a frame should be analyzed.
#[derive(Debug, Clone)]
pub struct Scheduler {
    threshold: f32,
    coalesce: Duration,
    previous: Option<MotionSignature>,
    last_analyzed_at: Option<Duration>,
    stats: SchedulerStats,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new(DEFAULT_MOTION_THRESHOLD, DEFAULT_COALESCE)
    }
}

impl Scheduler {
    /// A scheduler with an explicit threshold and coalescing window.
    ///
    /// `threshold` is clamped to `0.0..=1.0`. A zero threshold analyzes every
    /// unprotected frame, subject to coalescing.
    pub fn new(threshold: f32, coalesce: Duration) -> Self {
        Self {
            threshold: threshold.clamp(0.0, 1.0),
            coalesce,
            previous: None,
            last_analyzed_at: None,
            stats: SchedulerStats::default(),
        }
    }

    /// The motion threshold.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Sets the motion threshold, clamped to `0.0..=1.0`.
    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold.clamp(0.0, 1.0);
    }

    /// The coalescing window.
    pub fn coalesce(&self) -> Duration {
        self.coalesce
    }

    /// Sets the coalescing window. A zero window disables coalescing.
    pub fn set_coalesce(&mut self, coalesce: Duration) {
        self.coalesce = coalesce;
    }

    /// Content-free counters.
    pub fn stats(&self) -> SchedulerStats {
        self.stats
    }

    /// Returns `true` when `frame` should be analyzed.
    pub fn should_analyze(&mut self, frame: &VisionFrame<'_>) -> bool {
        self.decide(frame).is_none()
    }

    /// The reason a frame was skipped, or `None` when it should be analyzed.
    ///
    /// This is the whole decision. It updates scheduler state only for
    /// unprotected frames.
    pub fn decide(&mut self, frame: &VisionFrame<'_>) -> Option<SkipReason> {
        self.stats.offered += 1;

        if frame.protected {
            self.stats.protected += 1;
            return Some(SkipReason::Protected);
        }

        let signature = MotionSignature::from_pixels(frame.pixels);
        let motion = self
            .previous
            .as_ref()
            .map_or(1.0, |previous| signature.score_against(previous));
        self.previous = Some(signature);

        if motion < self.threshold {
            self.stats.r#static += 1;
            return Some(SkipReason::Static);
        }

        let now = frame.meta.captured_at;
        let coalesced = self
            .last_analyzed_at
            .is_some_and(|last| now.saturating_sub(last) < self.coalesce);
        if coalesced {
            self.stats.coalesced += 1;
            return Some(SkipReason::Coalesced);
        }

        self.last_analyzed_at = Some(now);
        self.stats.analyzed += 1;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curbcut_capture::ring::FrameMeta;
    use curbcut_core::id::FrameId;

    fn meta(id: u64, at_ms: u64) -> FrameMeta {
        FrameMeta {
            id: FrameId(id),
            captured_at: Duration::from_millis(at_ms),
            width: 10,
            height: 10,
            hash: [0u8; 32],
        }
    }

    fn solid(pixel: u8) -> Vec<u8> {
        vec![pixel; 256]
    }

    fn gradient(seed: u8) -> Vec<u8> {
        (0..256)
            .map(|index| (index as u8).wrapping_add(seed))
            .collect()
    }

    fn partial_change() -> Vec<u8> {
        let mut pixels = vec![0u8; 256];
        pixels[0] = 1;
        pixels
    }

    /// A frame over a temporary buffer. The buffer must live for the statement.
    fn frame(id: u64, at_ms: u64, pixels: &[u8]) -> VisionFrame<'_> {
        VisionFrame::new(meta(id, at_ms), pixels)
    }

    #[test]
    fn the_first_frame_is_analyzed() {
        let mut scheduler = Scheduler::default();
        assert!(scheduler.should_analyze(&frame(1, 0, &solid(1))));
        assert_eq!(scheduler.stats().analyzed, 1);
    }

    #[test]
    fn a_static_frame_is_skipped() {
        let mut scheduler = Scheduler::default();
        assert!(scheduler.should_analyze(&frame(1, 0, &solid(1))));
        assert!(!scheduler.should_analyze(&frame(2, 16, &solid(1))));
        assert!(!scheduler.should_analyze(&frame(3, 32, &solid(1))));
        assert_eq!(scheduler.stats().r#static, 2);
        assert_eq!(scheduler.stats().analyzed, 1);
    }

    #[test]
    fn a_moving_frame_outside_the_window_is_analyzed() {
        let mut scheduler = Scheduler::default();
        assert!(scheduler.should_analyze(&frame(1, 0, &solid(1))));
        assert!(scheduler.should_analyze(&frame(2, 1000, &gradient(2))));
        assert_eq!(scheduler.stats().analyzed, 2);
    }

    #[test]
    fn a_burst_is_coalesced() {
        let mut scheduler = Scheduler::default();
        assert!(scheduler.should_analyze(&frame(1, 0, &solid(1))));
        for id in 2..=5 {
            let at = id * 50;
            assert!(
                !scheduler.should_analyze(&frame(id, at, &gradient(id as u8))),
                "frame {id} should be coalesced"
            );
        }
        assert_eq!(scheduler.stats().coalesced, 4);
        assert_eq!(scheduler.stats().analyzed, 1);
        assert!(scheduler.should_analyze(&frame(6, 1000, &gradient(6))));
        assert_eq!(scheduler.stats().analyzed, 2);
    }

    #[test]
    fn a_protected_frame_is_never_analyzed_and_changes_no_state() {
        let mut scheduler = Scheduler::default();
        let pixels = solid(1);
        let protected = frame(1, 0, &pixels).protected();
        assert!(!scheduler.should_analyze(&protected));
        assert_eq!(scheduler.stats().protected, 1);
        assert_eq!(scheduler.stats().analyzed, 0);
        assert!(scheduler.previous.is_none(), "state must not advance");

        assert!(scheduler.should_analyze(&frame(2, 16, &solid(2))));
    }

    #[test]
    fn a_protected_frame_after_an_analyzed_frame_changes_no_state() {
        let mut scheduler = Scheduler::default();
        assert!(scheduler.should_analyze(&frame(1, 0, &solid(1))));
        let previous = scheduler.previous.clone();
        let last_analyzed_at = scheduler.last_analyzed_at;

        let pixels = solid(1);
        let protected = frame(2, 16, &pixels).protected();
        assert!(!scheduler.should_analyze(&protected));

        assert_eq!(scheduler.previous, previous, "previous must not advance");
        assert_eq!(
            scheduler.last_analyzed_at, last_analyzed_at,
            "the coalescing window must not move"
        );
        assert_eq!(scheduler.stats().protected, 1);
        assert_eq!(scheduler.stats().analyzed, 1);
    }

    #[test]
    fn motion_below_the_threshold_is_static() {
        let mut scheduler = Scheduler::new(0.9, Duration::from_millis(1));
        assert!(scheduler.should_analyze(&frame(1, 0, &solid(0))));
        assert!(!scheduler.should_analyze(&frame(2, 16, &partial_change())));
        assert_eq!(scheduler.stats().r#static, 1);
    }

    #[test]
    fn a_zero_threshold_analyzes_every_unprotected_frame() {
        let mut scheduler = Scheduler::new(0.0, Duration::ZERO);
        assert!(scheduler.should_analyze(&frame(1, 0, &solid(1))));
        assert!(scheduler.should_analyze(&frame(2, 1, &solid(1))));
        assert_eq!(scheduler.stats().analyzed, 2);
    }

    #[test]
    fn the_threshold_is_clamped() {
        let scheduler = Scheduler::new(7.5, DEFAULT_COALESCE);
        assert_eq!(scheduler.threshold(), 1.0);
        let scheduler = Scheduler::new(-1.0, DEFAULT_COALESCE);
        assert_eq!(scheduler.threshold(), 0.0);
    }

    #[test]
    fn decisions_are_deterministic() {
        let run = || {
            let mut scheduler = Scheduler::default();
            let decisions = vec![
                scheduler.should_analyze(&frame(1, 0, &solid(1))),
                scheduler.should_analyze(&frame(2, 16, &solid(1))),
                scheduler.should_analyze(&frame(3, 50, &gradient(3))),
                scheduler.should_analyze(&frame(4, 1000, &gradient(4))),
            ];
            decisions
        };
        assert_eq!(run(), run());
        assert_eq!(run(), vec![true, false, false, true]);
    }

    #[test]
    fn decide_reports_the_skip_reason() {
        let mut scheduler = Scheduler::default();
        assert_eq!(scheduler.decide(&frame(1, 0, &solid(1))), None);
        assert_eq!(
            scheduler.decide(&frame(2, 16, &solid(1))),
            Some(SkipReason::Static)
        );
        assert_eq!(
            scheduler.decide(&frame(3, 50, &gradient(3))),
            Some(SkipReason::Coalesced)
        );
        assert_eq!(
            scheduler.decide(&frame(4, 1000, &solid(1)).protected()),
            Some(SkipReason::Protected)
        );
    }

    #[test]
    fn counters_sum_to_the_offered_frames() {
        let mut scheduler = Scheduler::default();
        scheduler.should_analyze(&frame(1, 0, &solid(1)));
        scheduler.should_analyze(&frame(2, 16, &solid(1)));
        scheduler.should_analyze(&frame(3, 50, &gradient(3)));
        scheduler.should_analyze(&frame(4, 1000, &solid(1)).protected());
        let stats = scheduler.stats();
        assert_eq!(
            stats.analyzed + stats.r#static + stats.coalesced + stats.protected,
            stats.offered
        );
    }
}
