//! Optical character recognition.

use std::fmt;

use curbcut_core::error::Result;
use curbcut_core::types::Rect;

use crate::frame::VisionFrame;

/// Recognized text.
///
/// `text` is content: it must never be logged, dumped, or formatted by
/// `Debug`. Use [`OcrText::text`] to read it and [`OcrText::redacted`] for a
/// content-free view.
#[derive(Clone, PartialEq)]
pub struct OcrText {
    /// Screen rectangle of the recognized run.
    pub rect: Rect,
    /// Recognized text. Content: never log it.
    pub text: String,
    /// Confidence from the recognizer, from 0.0 to 1.0.
    pub confidence: f32,
}

impl OcrText {
    /// Creates a recognized run.
    pub fn new(rect: Rect, text: impl Into<String>, confidence: f32) -> Self {
        Self {
            rect,
            text: text.into(),
            confidence,
        }
    }

    /// A content-free view: the box, the length, and the confidence.
    pub fn redacted(&self) -> RedactedOcrText {
        RedactedOcrText {
            rect: self.rect,
            text_len: self.text.len(),
            confidence: self.confidence,
        }
    }
}

impl fmt::Debug for OcrText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OcrText")
            .field("rect", &self.rect)
            .field("text_len", &self.text.len())
            .field("confidence", &self.confidence)
            .finish()
    }
}

/// A content-free view of [`OcrText`].
#[derive(Debug, Clone, PartialEq)]
pub struct RedactedOcrText {
    /// Screen rectangle of the recognized run.
    pub rect: Rect,
    /// Length of the recognized text in bytes.
    pub text_len: usize,
    /// Confidence from the recognizer.
    pub confidence: f32,
}

/// Recognizes text in a frame.
///
/// Implementations run on-device. An recognizer must not log pixels or retain a
/// frame after `recognize` returns.
pub trait Ocr: fmt::Debug + Send {
    /// Recognizes text in `frame`.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError`](curbcut_core::error::CurbCutError) when the
    /// model cannot run. The pipeline treats a failure as "no text".
    fn recognize(&mut self, frame: &VisionFrame<'_>) -> Result<Vec<OcrText>>;
}

/// A recognizer that finds nothing. Useful as a placeholder and in tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoOcr;

impl Ocr for NoOcr {
    fn recognize(&mut self, _frame: &VisionFrame<'_>) -> Result<Vec<OcrText>> {
        Ok(Vec::new())
    }
}
