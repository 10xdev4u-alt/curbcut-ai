//! Integration tests for the vision pipeline through the public API.
//!
//! These drive the pipeline the way the capture adapter does: offer frames,
//! assert which ones reached a model, and assert that no content is exposed.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use curbcut_capture::ring::FrameMeta;
use curbcut_core::id::FrameId;
use curbcut_core::types::{Rect, Role};
use curbcut_vision::caption::{IconCaption, IconCaptioner};
use curbcut_vision::detector::{Detection, Detector};
use curbcut_vision::frame::VisionFrame;
use curbcut_vision::ocr::{Ocr, OcrText};
use curbcut_vision::pipeline::{VisionOutcome, VisionPipeline, VisionResult};
use curbcut_vision::scheduler::SkipReason;

fn solid(seed: u8) -> Vec<u8> {
    (0..256)
        .map(|index| (index as u8).wrapping_add(seed))
        .collect()
}

fn meta(id: u64, at_ms: u64) -> FrameMeta {
    FrameMeta {
        id: FrameId(id),
        captured_at: Duration::from_millis(at_ms),
        width: 16,
        height: 16,
        hash: [0u8; 32],
    }
}

/// A frame over a temporary buffer. The buffer lives for the statement.
macro_rules! frame {
    ($id:expr, $at:expr, $seed:expr) => {
        VisionFrame::new(meta($id, $at), &solid($seed))
    };
}

/// A detector that counts calls and records whether it ever saw a protected
/// frame. Cloning shares the counters.
#[derive(Debug, Clone, Default)]
struct CountingDetector {
    calls: Arc<AtomicUsize>,
    protected_seen: Arc<AtomicUsize>,
}

impl Detector for CountingDetector {
    fn detect(&mut self, frame: &VisionFrame<'_>) -> curbcut_core::error::Result<Vec<Detection>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if frame.protected {
            self.protected_seen.fetch_add(1, Ordering::SeqCst);
        }
        Ok(vec![Detection {
            rect: Rect::new(0, 0, 8, 8),
            role: Role::Image,
            confidence: 0.75,
        }])
    }
}

#[derive(Debug)]
struct NoopOcr;

impl Ocr for NoopOcr {
    fn recognize(&mut self, _frame: &VisionFrame<'_>) -> curbcut_core::error::Result<Vec<OcrText>> {
        Ok(vec![OcrText::new(Rect::new(0, 0, 8, 8), "read", 0.9)])
    }
}

#[derive(Debug)]
struct NoopCaptioner;

impl IconCaptioner for NoopCaptioner {
    fn caption(
        &mut self,
        _frame: &VisionFrame<'_>,
        detections: &[Detection],
    ) -> curbcut_core::error::Result<Vec<IconCaption>> {
        Ok(detections
            .iter()
            .map(|detection| IconCaption::new(detection.rect, "icon", 0.6))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pipeline() -> (VisionPipeline, CountingDetector) {
        let detector = CountingDetector::default();
        let handle = detector.clone();
        (
            VisionPipeline::new(
                Box::new(detector),
                Box::new(NoopOcr),
                Box::new(NoopCaptioner),
            ),
            handle,
        )
    }

    #[test]
    fn static_frames_never_reach_a_model() {
        let (mut pipeline, detector) = pipeline();

        assert!(matches!(
            pipeline.process(&frame!(1, 0, 1)),
            VisionOutcome::Analyzed(_)
        ));
        match pipeline.process(&frame!(2, 16, 1)) {
            VisionOutcome::Skipped(reason) => assert_eq!(reason, SkipReason::Static),
            other => panic!("expected skipped, got {other:?}"),
        }

        assert_eq!(detector.calls.load(Ordering::SeqCst), 1);
        assert_eq!(pipeline.stats().analyzed, 1);
        assert_eq!(pipeline.stats().r#static, 1);
    }

    #[test]
    fn motion_above_the_threshold_triggers_analysis() {
        let (mut pipeline, _detector) = pipeline();

        assert!(matches!(
            pipeline.process(&frame!(1, 0, 1)),
            VisionOutcome::Analyzed(_)
        ));
        assert!(matches!(
            pipeline.process(&frame!(2, 1000, 2)),
            VisionOutcome::Analyzed(_)
        ));
        assert_eq!(pipeline.stats().analyzed, 2);
    }

    #[test]
    fn a_burst_is_coalesced_into_one_analysis() {
        let (mut pipeline, _detector) = pipeline();

        assert!(matches!(
            pipeline.process(&frame!(1, 0, 1)),
            VisionOutcome::Analyzed(_)
        ));
        for id in 2..=8 {
            assert!(
                matches!(
                    pipeline.process(&frame!(id, id * 20, id as u8)),
                    VisionOutcome::Skipped(SkipReason::Coalesced)
                ),
                "frame {id} should be coalesced"
            );
        }
        assert_eq!(pipeline.stats().analyzed, 1);
        assert_eq!(pipeline.stats().coalesced, 7);
    }

    #[test]
    fn a_protected_frame_is_never_enqueued() {
        let (mut pipeline, detector) = pipeline();

        let pixels = solid(1);
        let protected = VisionFrame::new(meta(1, 0), &pixels).protected();
        match pipeline.process(&protected) {
            VisionOutcome::Skipped(reason) => assert_eq!(reason, SkipReason::Protected),
            other => panic!("expected skipped, got {other:?}"),
        }

        assert_eq!(detector.calls.load(Ordering::SeqCst), 0);
        assert_eq!(detector.protected_seen.load(Ordering::SeqCst), 0);
        assert_eq!(pipeline.stats().analyzed, 0);
    }

    #[test]
    fn the_scheduler_is_deterministic() {
        let run = || {
            let mut pipeline = VisionPipeline::passive();
            let mut outcomes = Vec::new();
            for id in 1..=12 {
                outcomes.push(pipeline.process(&frame!(id, id * 20, id as u8)));
            }
            outcomes
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn a_result_carries_no_content_in_its_debug() {
        let (mut pipeline, _detector) = pipeline();

        match pipeline.process(&frame!(1, 0, 1)) {
            VisionOutcome::Analyzed(result) => {
                let dbg = format!("{result:?}");
                assert!(!dbg.contains("read"), "{dbg}");
                assert!(!dbg.contains("icon"), "{dbg}");
                assert!(dbg.contains("text_len"), "{dbg}");
            }
            other => panic!("expected analyzed, got {other:?}"),
        }
    }

    #[test]
    fn recognized_text_and_captions_are_zeroized_when_the_result_drops() {
        curbcut_core::secure::reset_zeroized_on_drop();
        let (mut pipeline, _detector) = pipeline();

        let result = match pipeline.process(&frame!(1, 0, 1)) {
            VisionOutcome::Analyzed(result) => result,
            other => panic!("expected analyzed, got {other:?}"),
        };
        assert_eq!(result.texts[0].as_str(), "read");
        assert_eq!(result.captions[0].as_str(), "icon");
        assert_eq!(curbcut_core::secure::zeroized_on_drop(), 0);

        drop(result);
        assert_eq!(
            curbcut_core::secure::zeroized_on_drop(),
            2,
            "the recognized text and the caption must be zeroized on drop"
        );
    }

    #[test]
    fn the_passive_pipeline_returns_empty_results() {
        let mut pipeline = VisionPipeline::passive();
        match pipeline.process(&frame!(1, 0, 1)) {
            VisionOutcome::Analyzed(result) => assert!(result.is_empty()),
            other => panic!("expected analyzed, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_result_is_constructible() {
        let result = VisionResult::empty(FrameId(9));
        assert_eq!(result.frame_id, FrameId(9));
        assert!(result.is_empty());
    }
}
