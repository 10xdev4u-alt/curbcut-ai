//! The frame type the vision pipeline reasons about.

use std::fmt;

use curbcut_capture::ring::FrameMeta;
use curbcut_core::id::FrameId;

/// A frame offered to the vision pipeline.
///
/// Pixels are borrowed; the pipeline never retains them. `Debug` prints the
/// metadata and the pixel length, never the pixels.
#[derive(Clone)]
pub struct VisionFrame<'a> {
    /// Content-free metadata.
    pub meta: FrameMeta,
    /// Pixel bytes in the platform's capture format.
    pub pixels: &'a [u8],
    /// Whether the frame is protected.
    ///
    /// A protected frame is never enqueued for analysis and never reaches a
    /// model. The capture adapter sets this for regions the user marked as
    /// sensitive.
    pub protected: bool,
}

impl<'a> VisionFrame<'a> {
    /// A frame from metadata and borrowed pixels.
    pub fn new(meta: FrameMeta, pixels: &'a [u8]) -> Self {
        Self {
            meta,
            pixels,
            protected: false,
        }
    }

    /// Marks the frame as protected.
    pub fn protected(mut self) -> Self {
        self.protected = true;
        self
    }

    /// The frame id.
    pub fn id(&self) -> FrameId {
        self.meta.id
    }
}

impl fmt::Debug for VisionFrame<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VisionFrame")
            .field("meta", &self.meta)
            .field("pixel_len", &self.pixels.len())
            .field("protected", &self.protected)
            .finish()
    }
}
