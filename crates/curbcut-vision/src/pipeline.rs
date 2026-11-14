//! The vision pipeline: scheduler, detector, OCR, and captioner.
//!
//! [`VisionPipeline::process`] is the single entry point. It asks the
//! [`Scheduler`] whether a frame is worth analyzing, then runs the three model
//! stages. A stage that fails yields an empty result and records the stage name;
//! it never panics and never logs content.
//!
//! Every result type in this crate has a content-free [`Debug`](std::fmt::Debug):
//! `OcrText` and `IconCaption` print lengths, not text.

use curbcut_core::id::FrameId;

use crate::caption::{IconCaption, IconCaptioner, NoCaptioner};
use crate::detector::{Detection, Detector, NoDetector};
use crate::frame::VisionFrame;
use crate::ocr::{NoOcr, Ocr, OcrText};
use crate::scheduler::{Scheduler, SchedulerStats, SkipReason};

/// What the pipeline did with a frame.
#[derive(Debug, Clone, PartialEq)]
pub enum VisionOutcome {
    /// The scheduler decided not to spend a model call.
    Skipped(SkipReason),
    /// The frame was analyzed.
    Analyzed(VisionResult),
}

/// The content-free result of analyzing one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct VisionResult {
    /// The frame that was analyzed.
    pub frame_id: FrameId,
    /// Detected elements.
    pub detections: Vec<Detection>,
    /// Recognized text runs.
    pub texts: Vec<OcrText>,
    /// Captions for icons and images.
    pub captions: Vec<IconCaption>,
    /// Stages that failed, named. Never carries content.
    pub errors: Vec<&'static str>,
}

impl VisionResult {
    /// A result with no detections, text, captions, or errors.
    pub fn empty(frame_id: FrameId) -> Self {
        Self {
            frame_id,
            detections: Vec::new(),
            texts: Vec::new(),
            captions: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Whether every stage returned nothing.
    pub fn is_empty(&self) -> bool {
        self.detections.is_empty()
            && self.texts.is_empty()
            && self.captions.is_empty()
            && self.errors.is_empty()
    }
}

/// Runs the three model stages behind a scheduler.
pub struct VisionPipeline {
    detector: Box<dyn Detector>,
    ocr: Box<dyn Ocr>,
    captioner: Box<dyn IconCaptioner>,
    scheduler: Scheduler,
}

impl VisionPipeline {
    /// A pipeline over the three model backends.
    pub fn new(
        detector: Box<dyn Detector>,
        ocr: Box<dyn Ocr>,
        captioner: Box<dyn IconCaptioner>,
    ) -> Self {
        Self {
            detector,
            ocr,
            captioner,
            scheduler: Scheduler::default(),
        }
    }

    /// A pipeline with no backends. Everything is empty; useful as a placeholder
    /// and in tests.
    pub fn passive() -> Self {
        Self::new(Box::new(NoDetector), Box::new(NoOcr), Box::new(NoCaptioner))
    }

    /// The scheduler.
    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    /// The scheduler, mutably.
    pub fn scheduler_mut(&mut self) -> &mut Scheduler {
        &mut self.scheduler
    }

    /// Content-free scheduler counters.
    pub fn stats(&self) -> SchedulerStats {
        self.scheduler.stats()
    }

    /// Processes one frame.
    ///
    /// A skipped frame runs no model and returns [`VisionOutcome::Skipped`]. An
    /// analyzed frame returns [`VisionOutcome::Analyzed`]; a failing stage
    /// contributes an empty result and its name to [`VisionResult::errors`].
    pub fn process(&mut self, frame: &VisionFrame<'_>) -> VisionOutcome {
        match self.scheduler.decide(frame) {
            Some(reason) => VisionOutcome::Skipped(reason),
            None => VisionOutcome::Analyzed(self.analyze(frame)),
        }
    }

    fn analyze(&mut self, frame: &VisionFrame<'_>) -> VisionResult {
        let mut result = VisionResult::empty(frame.meta.id);

        match self.detector.detect(frame) {
            Ok(detections) => result.detections = detections,
            Err(_) => result.errors.push("detector"),
        }

        match self.ocr.recognize(frame) {
            Ok(texts) => result.texts = texts,
            Err(_) => result.errors.push("ocr"),
        }

        match self.captioner.caption(frame, &result.detections) {
            Ok(captions) => result.captions = captions,
            Err(_) => result.errors.push("captioner"),
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curbcut_core::error::{CurbCutError, Result};
    use curbcut_core::types::{Rect, Role};

    fn solid(pixel: u8) -> Vec<u8> {
        vec![pixel; 256]
    }

    fn gradient(seed: u8) -> Vec<u8> {
        (0..256)
            .map(|index| (index as u8).wrapping_add(seed))
            .collect()
    }

    /// A frame over a temporary buffer. The buffer must live for the statement.
    fn frame(id: u64, at_ms: u64, pixels: &[u8]) -> VisionFrame<'_> {
        VisionFrame::new(meta(id, at_ms), pixels)
    }

    fn meta(id: u64, at_ms: u64) -> curbcut_capture::ring::FrameMeta {
        curbcut_capture::ring::FrameMeta {
            id: FrameId(id),
            captured_at: std::time::Duration::from_millis(at_ms),
            width: 10,
            height: 10,
            hash: [0u8; 32],
        }
    }

    #[derive(Debug)]
    struct StubDetector {
        fail: bool,
    }

    impl Detector for StubDetector {
        fn detect(&mut self, _frame: &VisionFrame<'_>) -> Result<Vec<Detection>> {
            if self.fail {
                return Err(CurbCutError::NotSupported {
                    feature: "detector".to_string(),
                });
            }
            Ok(vec![Detection {
                rect: Rect::new(0, 0, 1, 1),
                role: Role::Button,
                confidence: 0.5,
            }])
        }
    }

    #[derive(Debug)]
    struct StubOcr {
        fail: bool,
    }

    impl Ocr for StubOcr {
        fn recognize(&mut self, _frame: &VisionFrame<'_>) -> Result<Vec<OcrText>> {
            if self.fail {
                return Err(CurbCutError::NotSupported {
                    feature: "ocr".to_string(),
                });
            }
            Ok(vec![OcrText::new(Rect::new(0, 0, 1, 1), "stub", 0.5)])
        }
    }

    #[derive(Debug)]
    struct StubCaptioner;

    impl IconCaptioner for StubCaptioner {
        fn caption(
            &mut self,
            _frame: &VisionFrame<'_>,
            detections: &[Detection],
        ) -> Result<Vec<IconCaption>> {
            Ok(detections
                .iter()
                .map(|detection| IconCaption::new(detection.rect, "stub", 0.5))
                .collect())
        }
    }

    fn pipeline() -> VisionPipeline {
        VisionPipeline::new(
            Box::new(StubDetector { fail: false }),
            Box::new(StubOcr { fail: false }),
            Box::new(StubCaptioner),
        )
    }

    #[test]
    fn an_analyzed_frame_runs_all_three_stages() {
        let mut pipeline = pipeline();
        match pipeline.process(&frame(1, 0, &solid(1))) {
            VisionOutcome::Analyzed(result) => {
                assert_eq!(result.frame_id, FrameId(1));
                assert_eq!(result.detections.len(), 1);
                assert_eq!(result.texts.len(), 1);
                assert_eq!(result.captions.len(), 1);
                assert!(result.errors.is_empty());
            }
            other => panic!("expected analyzed, got {other:?}"),
        }
    }

    #[test]
    fn a_static_frame_is_skipped_and_runs_no_stage() {
        let mut pipeline = pipeline();
        assert!(matches!(
            pipeline.process(&frame(1, 0, &solid(1))),
            VisionOutcome::Analyzed(_)
        ));
        match pipeline.process(&frame(2, 16, &solid(1))) {
            VisionOutcome::Skipped(reason) => assert_eq!(reason, SkipReason::Static),
            other => panic!("expected skipped, got {other:?}"),
        }
        assert_eq!(pipeline.stats().analyzed, 1);
    }

    #[test]
    fn a_protected_frame_is_never_analyzed() {
        let mut pipeline = pipeline();
        let pixels = solid(1);
        let protected = frame(1, 0, &pixels).protected();
        match pipeline.process(&protected) {
            VisionOutcome::Skipped(reason) => assert_eq!(reason, SkipReason::Protected),
            other => panic!("expected skipped, got {other:?}"),
        }
        assert_eq!(pipeline.stats().analyzed, 0);
    }

    #[test]
    fn a_burst_is_coalesced() {
        let mut pipeline = pipeline();
        assert!(matches!(
            pipeline.process(&frame(1, 0, &solid(1))),
            VisionOutcome::Analyzed(_)
        ));
        // Moving frames inside the coalescing window are skipped without a
        // model call.
        assert!(matches!(
            pipeline.process(&frame(2, 50, &gradient(2))),
            VisionOutcome::Skipped(SkipReason::Coalesced)
        ));
        assert!(matches!(
            pipeline.process(&frame(3, 100, &gradient(3))),
            VisionOutcome::Skipped(SkipReason::Coalesced)
        ));
        assert_eq!(pipeline.stats().analyzed, 1);
        assert_eq!(pipeline.stats().coalesced, 2);
    }

    #[test]
    fn a_failing_stage_records_its_name_and_the_others_still_run() {
        let mut pipeline = VisionPipeline::new(
            Box::new(StubDetector { fail: true }),
            Box::new(StubOcr { fail: false }),
            Box::new(StubCaptioner),
        );
        match pipeline.process(&frame(1, 0, &solid(1))) {
            VisionOutcome::Analyzed(result) => {
                assert_eq!(result.errors, vec!["detector"]);
                assert!(result.detections.is_empty());
                assert_eq!(result.texts.len(), 1);
            }
            other => panic!("expected analyzed, got {other:?}"),
        }
    }

    #[test]
    fn every_stage_can_fail_and_the_pipeline_still_returns() {
        let mut pipeline = VisionPipeline::new(
            Box::new(StubDetector { fail: true }),
            Box::new(StubOcr { fail: true }),
            Box::new(StubCaptioner),
        );
        match pipeline.process(&frame(1, 0, &solid(1))) {
            VisionOutcome::Analyzed(result) => {
                assert_eq!(result.errors, vec!["detector", "ocr"]);
                assert!(!result.is_empty(), "errors are not empty");
            }
            other => panic!("expected analyzed, got {other:?}"),
        }
    }

    #[test]
    fn the_passive_pipeline_returns_empty_results() {
        let mut pipeline = VisionPipeline::passive();
        match pipeline.process(&frame(1, 0, &solid(1))) {
            VisionOutcome::Analyzed(result) => assert!(result.is_empty()),
            other => panic!("expected analyzed, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_result_is_only_empty_when_everything_is_empty() {
        let mut result = VisionResult::empty(FrameId(1));
        assert!(result.is_empty());
        result.errors.push("ocr");
        assert!(!result.is_empty());
    }

    #[test]
    fn debug_is_content_free() {
        let text = OcrText::new(Rect::new(0, 0, 1, 1), "secret-content", 0.5);
        let caption = IconCaption::new(Rect::new(0, 0, 1, 1), "secret-caption", 0.5);
        let dbg = format!("{text:?} {caption:?}");
        assert!(!dbg.contains("secret-content"), "{dbg}");
        assert!(!dbg.contains("secret-caption"), "{dbg}");
        assert!(dbg.contains("text_len"), "{dbg}");
        assert!(dbg.contains("caption_len"), "{dbg}");
    }
}
