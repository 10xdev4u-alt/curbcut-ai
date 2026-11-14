//! Element detection.

use std::fmt;

use curbcut_core::error::Result;
use curbcut_core::types::{Rect, Role};

use crate::frame::VisionFrame;

/// A detected element. Content-free: a box, a role, and a confidence.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    /// Screen rectangle.
    pub rect: Rect,
    /// Semantic role.
    pub role: Role,
    /// Confidence from the detector, from 0.0 to 1.0.
    pub confidence: f32,
}

/// Detects interactive and reading elements in a frame.
///
/// Implementations run on-device. A detector must not log pixels or retain a
/// frame after `detect` returns.
pub trait Detector: fmt::Debug + Send {
    /// Detects elements in `frame`.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError`](curbcut_core::error::CurbCutError) when the
    /// model cannot run. The pipeline treats a failure as "no detections".
    fn detect(&mut self, frame: &VisionFrame<'_>) -> Result<Vec<Detection>>;
}

/// A detector that finds nothing. Useful as a placeholder and in tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoDetector;

impl Detector for NoDetector {
    fn detect(&mut self, _frame: &VisionFrame<'_>) -> Result<Vec<Detection>> {
        Ok(Vec::new())
    }
}
