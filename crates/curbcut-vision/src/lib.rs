//! ONNX inference, detection, OCR, layout, and motion scheduling.
//!
//! The pipeline is [`pipeline::VisionPipeline`]: a [`scheduler::Scheduler`]
//! decides which frames are worth a model call, then [`detector::Detector`],
//! [`ocr::Ocr`], and [`caption::IconCaptioner`] run. Every model call is
//! on-device and every result type has a content-free [`Debug`](std::fmt::Debug).
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
pub mod frame;
pub mod motion;
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
