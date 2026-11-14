//! Integration tests for the frame ring buffer through the public API.

use std::time::Duration;

use curbcut_capture::ring::{
    FrameBuffer, FrameMeta, MlockBackend, MlockStatus, NoMlock, RingBuffer, RingStats, GUARD_BYTE,
    MAX_FRAME_BYTES, RING_SECONDS,
};
use curbcut_core::id::FrameId;

/// Non-guard pattern for the zeroization tests.
const PATTERN: u8 = 0xA5;

#[derive(Debug, Default)]
struct RecordingMlock {
    locked_bytes: std::sync::atomic::AtomicUsize,
}

impl MlockBackend for RecordingMlock {
    fn lock(&self, data: &[u8]) -> MlockStatus {
        self.locked_bytes
            .fetch_add(data.len(), std::sync::atomic::Ordering::Relaxed);
        MlockStatus::Locked
    }

    fn unlock(&self, data: &[u8]) {
        self.locked_bytes
            .fetch_sub(data.len(), std::sync::atomic::Ordering::Relaxed);
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
    let mut buffer = FrameBuffer::new(512).expect("buffer");
    buffer.as_mut_slice().fill(PATTERN);
    buffer.zeroize();
    assert!(buffer.is_zeroized());
    assert!(buffer.as_slice().iter().all(|byte| *byte == GUARD_BYTE));
    assert_eq!(buffer.len(), 512, "length is retained for reuse");
}

#[test]
fn an_empty_frame_is_rejected() {
    let mut ring = RingBuffer::new(4, 1024, Box::new(NoMlock)).expect("ring");
    let err = ring.push(&[], meta(1)).expect_err("empty frame");
    assert!(err.to_string().contains("empty"), "{err}");
    assert!(ring.is_empty());
}

#[test]
fn a_zero_frame_rate_is_rejected() {
    let err = RingBuffer::with_fps(0, 10, 1024, Box::new(NoMlock)).expect_err("zero fps");
    assert!(err.to_string().contains("fps"), "{err}");
}

#[test]
fn a_zero_duration_is_rejected() {
    let err = RingBuffer::with_fps(30, 0, 1024, Box::new(NoMlock)).expect_err("zero seconds");
    assert!(err.to_string().contains("capacity"), "{err}");
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

#[derive(Debug, Default)]
struct ProbeState {
    unlocks: std::sync::atomic::AtomicUsize,
    dirty: std::sync::atomic::AtomicUsize,
}

#[derive(Debug, Default, Clone)]
struct ZeroizeProbe {
    state: std::sync::Arc<ProbeState>,
}

impl MlockBackend for ZeroizeProbe {
    fn lock(&self, _data: &[u8]) -> MlockStatus {
        MlockStatus::Locked
    }

    fn unlock(&self, data: &[u8]) {
        self.state
            .unlocks
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if !data.iter().all(|byte| *byte == GUARD_BYTE) {
            self.state
                .dirty
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

#[test]
fn an_evicted_frame_is_scrubbed_before_unlock() {
    let probe = ZeroizeProbe::default();
    let state = probe.state.clone();
    let mut ring = RingBuffer::new(2, 1024, Box::new(probe)).expect("ring");
    for id in 0..4u64 {
        ring.push(&[id as u8; 8], meta(id)).expect("push");
    }
    assert_eq!(state.unlocks.load(std::sync::atomic::Ordering::Relaxed), 2);
    assert_eq!(
        state.dirty.load(std::sync::atomic::Ordering::Relaxed),
        0,
        "the backend only ever saw guard bytes"
    );
}
