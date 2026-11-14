//! Integration tests for the frame ring buffer through the public API.

use std::time::Duration;

use curbcut_capture::ring::{
    FrameBuffer, FrameMeta, MlockBackend, MlockStatus, NoMlock, RingBuffer, RingStats, GUARD_BYTE,
    MAX_FRAME_BYTES, RING_SECONDS, TEST_PATTERN,
};
use curbcut_core::id::FrameId;

#[derive(Debug, Default)]
struct RecordingMlock {
    locked_bytes: std::sync::atomic::AtomicUsize,
}

impl MlockBackend for RecordingMlock {
    fn lock(&self, len: usize) -> MlockStatus {
        self.locked_bytes
            .fetch_add(len, std::sync::atomic::Ordering::Relaxed);
        MlockStatus::Locked
    }

    fn unlock(&self, len: usize) {
        self.locked_bytes
            .fetch_sub(len, std::sync::atomic::Ordering::Relaxed);
    }
}

fn meta(id: u64) -> FrameMeta {
    FrameMeta {
        id: FrameId(id),
        captured_at: Duration::from_millis(id),
        width: 4,
        height: 4,
        hash: [0u8; 32],
    }
}

#[test]
fn a_released_buffer_is_overwritten_with_the_guard_byte() {
    let mut buffer = FrameBuffer::with_fill(512, TEST_PATTERN);
    buffer.zeroize();
    assert!(buffer.is_zeroized());
    assert!(buffer.as_slice().iter().all(|byte| *byte == GUARD_BYTE));
    assert_eq!(buffer.len(), 512, "length is retained for reuse");
}

#[test]
fn a_frame_larger_than_the_budget_is_rejected() {
    let mut ring = RingBuffer::new(4, 64, Box::new(NoMlock)).expect("ring");
    assert!(ring.push(&[0u8; 65], meta(1)).is_err());
    assert_eq!(
        ring.stats(),
        RingStats {
            capacity: 4,
            budget_bytes: 64,
            ..RingStats::default()
        }
    );
}

#[test]
fn a_frame_larger_than_the_hard_limit_is_rejected() {
    let mut ring = RingBuffer::new(4, MAX_FRAME_BYTES, Box::new(NoMlock)).expect("ring");
    // Allocate just over the hard limit without materialising it first.
    let len = MAX_FRAME_BYTES.saturating_add(1);
    let err = ring.push(&vec![0u8; len], meta(1)).expect_err("over limit");
    assert!(err.to_string().contains("frame length"), "{err}");
}

#[test]
fn the_default_ring_is_two_seconds_at_thirty_fps() {
    let ring =
        RingBuffer::with_fps(30, RING_SECONDS, 64 * 1024 * 1024, Box::new(NoMlock)).expect("ring");
    assert_eq!(ring.capacity(), 60);
}

#[test]
fn mlock_status_is_recorded_and_never_fatal() {
    let backend = Box::new(RecordingMlock::default());
    let mut ring = RingBuffer::new(2, 1024, backend).expect("ring");
    ring.push(&[1u8; 8], meta(1)).expect("push");
    assert_eq!(ring.mlock_locked_bytes(), 8);

    let failing = RingBuffer::new(2, 1024, Box::new(NoMlock)).expect("ring");
    assert_eq!(failing.mlock_locked_bytes(), 0, "unsupported is non-fatal");
}

#[test]
fn stats_track_the_ring_over_a_burst() {
    let mut ring = RingBuffer::new(4, 4096, Box::new(NoMlock)).expect("ring");
    for id in 0..10 {
        ring.push(&[0u8; 16], meta(id)).expect("push");
    }
    let stats = ring.stats();
    assert_eq!(stats.capacity, 4);
    assert_eq!(stats.occupied, 4);
    assert_eq!(stats.resident_bytes, 64);
    assert_eq!(stats.pushed, 10);
    assert_eq!(stats.overwritten, 6);
    assert_eq!(stats.budget_bytes, 4096);
}

#[test]
fn get_and_latest_are_content_free_in_metadata() {
    let mut ring = RingBuffer::new(4, 1024, Box::new(NoMlock)).expect("ring");
    let id = ring.push(&[1, 2, 3], meta(7)).expect("push");
    let (bytes, frame_meta) = ring.get(id).expect("frame");
    assert_eq!(bytes, &[1, 2, 3]);
    assert_eq!(frame_meta.id, id);
    assert_eq!(frame_meta.width, 4);
    assert_eq!(ring.latest().expect("latest").1.id, id);
}
