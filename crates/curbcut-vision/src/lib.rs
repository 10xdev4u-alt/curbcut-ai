//! ONNX inference, detection, OCR, layout, and motion scheduling.
//!
//! The pipeline is [`pipeline::VisionPipeline`]: a [`scheduler::Scheduler`]
//! decides which frames are worth a model call, then [`detector::Detector`],
//! [`ocr::Ocr`], and [`caption::IconCaptioner`] run. Every model call is
//! on-device and every result type has a content-free [`Debug`](std::fmt::Debug).
//!
//! [`motion_report`] classifies a region's changed-sample fraction, rectangle,
//! and a11y roles as static, carousel, autoplay, or countdown, and reports a
//! region once it has moved for long enough. See
//! [`../../../docs/MOTION_DETECTION.md`](../../../docs/MOTION_DETECTION.md).
//!
//! [`layout`] divides a viewport into header, nav, main, aside, footer, modal,
//! and floating regions and produces a stable reading order. See
//! [`../../../docs/LAYOUT_ANALYSIS.md`](../../../docs/LAYOUT_ANALYSIS.md).
//!
//! [`dirty`] decides which frames are worth a model call: a downscaled grid of
//! cell means, merged changed rectangles, and a cooldown so a moving carousel
//! triggers one re-analysis rather than one per frame. See
//! [`../../../docs/DIRTY_RECT.md`](../../../docs/DIRTY_RECT.md).
//!
//! See [`../../../docs/VISION.md`](../../../docs/VISION.md) and
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-vision";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod caption;
pub mod detector;
pub mod dirty;
pub mod frame;
pub mod layout;
pub mod motion;
pub mod motion_report;
pub mod ocr;
pub mod pipeline;
pub mod scheduler;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-vision");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
