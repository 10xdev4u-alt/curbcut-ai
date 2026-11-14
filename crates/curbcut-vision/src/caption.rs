//! Icon captioning.

use std::fmt;

use curbcut_core::error::Result;
use curbcut_core::types::Rect;

use crate::detector::Detection;
use crate::frame::VisionFrame;

/// A caption for an icon or image.
///
/// `caption` is content: it must never be logged, dumped, or formatted by
/// `Debug`. Use [`IconCaption::redacted`] for a content-free view.
#[derive(Clone, PartialEq)]
pub struct IconCaption {
    /// Screen rectangle of the icon.
    pub rect: Rect,
    /// Caption text. Content: never log it.
    pub caption: String,
    /// Confidence from the captioner, from 0.0 to 1.0.
    pub confidence: f32,
}

impl IconCaption {
    /// Creates a caption.
    pub fn new(rect: Rect, caption: impl Into<String>, confidence: f32) -> Self {
        Self {
            rect,
            caption: caption.into(),
            confidence,
        }
    }

    /// A content-free view: the box, the length, and the confidence.
    pub fn redacted(&self) -> RedactedIconCaption {
        RedactedIconCaption {
            rect: self.rect,
            caption_len: self.caption.len(),
            confidence: self.confidence,
        }
    }
}

impl fmt::Debug for IconCaption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IconCaption")
            .field("rect", &self.rect)
            .field("caption_len", &self.caption.len())
            .field("confidence", &self.confidence)
            .finish()
    }
}

/// A content-free view of [`IconCaption`].
#[derive(Debug, Clone, PartialEq)]
pub struct RedactedIconCaption {
    /// Screen rectangle of the icon.
    pub rect: Rect,
    /// Length of the caption in bytes.
    pub caption_len: usize,
    /// Confidence from the captioner.
    pub confidence: f32,
}

/// Captions icons and images so a screen reader can announce them.
///
/// Implementations run on-device. A captioner must not log pixels or retain a
/// frame after `caption` returns.
pub trait IconCaptioner: fmt::Debug + Send {
    /// Captions the icons among `detections`.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError`](curbcut_core::error::CurbCutError) when the
    /// model cannot run. The pipeline treats a failure as "no captions".
    fn caption(
        &mut self,
        frame: &VisionFrame<'_>,
        detections: &[Detection],
    ) -> Result<Vec<IconCaption>>;
}

/// A captioner that produces nothing. Useful as a placeholder and in tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoCaptioner;

impl IconCaptioner for NoCaptioner {
    fn caption(
        &mut self,
        _frame: &VisionFrame<'_>,
        _detections: &[Detection],
    ) -> Result<Vec<IconCaption>> {
        Ok(Vec::new())
    }
}
