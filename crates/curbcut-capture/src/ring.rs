//! Zeroizing frame ring buffer with best-effort `mlock`.
//!
//! Frames are the only place pixels exist, and they never leave the process.
//! Every buffer is overwritten with [`GUARD_BYTE`] when released or reused, so
//! a stale frame cannot be read back after it leaves the ring.
//!
//! `mlock` is attempted through an [`MlockBackend`]. Safe, dependency-free Rust
//! cannot issue the `mlock(2)` syscall, so the default backend is [`NoMlock`]
//! and reports [`MlockStatus::Unsupported`]. A platform adapter supplies a real
//! backend; the ring treats every non-`Locked` result as non-fatal.

use std::fmt;
use std::time::Duration;

use curbcut_core::error::{CurbCutError, Result};
use curbcut_core::id::FrameId;

/// Default ring length in seconds.
pub const RING_SECONDS: u32 = 2;
/// Default capture rate in frames per second.
pub const DEFAULT_FPS: u32 = 30;
/// Largest accepted frame: 64 MiB, enough for a 4K RGBA frame.
pub const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;
/// Byte written over a released frame.
pub const GUARD_BYTE: u8 = 0;

/// Pattern used by the guard-byte test.
#[cfg(test)]
pub(crate) const TEST_PATTERN: u8 = 0xA5;

// Counts buffers dropped on the current thread, for the zeroization guard
// test. Thread-local so parallel tests cannot race on a process-global
// counter.
#[cfg(test)]
thread_local! {
    pub(crate) static ZEROIZED_ON_DROP: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

/// An owned frame buffer that zeroes itself on release.
pub struct FrameBuffer {
    data: Vec<u8>,
}

impl FrameBuffer {
    /// Allocates a buffer of `len` bytes, prefilled with [`GUARD_BYTE`].
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::InvalidInput`] when `len` exceeds
    /// [`MAX_FRAME_BYTES`].
    pub fn new(len: usize) -> Result<Self> {
        if len > MAX_FRAME_BYTES {
            return Err(CurbCutError::InvalidInput {
                field: format!("frame length exceeds {}", MAX_FRAME_BYTES),
            });
        }
        Ok(Self {
            data: vec![GUARD_BYTE; len],
        })
    }

    /// Allocates a buffer prefilled with `fill`. Test helper; production code
    /// uses [`FrameBuffer::new`], which starts from [`GUARD_BYTE`].
    #[cfg(test)]
    pub fn with_fill(len: usize, fill: u8) -> Self {
        Self {
            data: vec![fill; len],
        }
    }

    /// Overwrites every byte with [`GUARD_BYTE`].
    pub fn zeroize(&mut self) {
        self.data.fill(GUARD_BYTE);
    }

    /// Whether every byte is the guard byte.
    pub fn is_zeroized(&self) -> bool {
        self.data.iter().all(|byte| *byte == GUARD_BYTE)
    }

    /// The frame bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// The frame bytes, mutably.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Buffer length in bytes.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the buffer has length zero.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Drop for FrameBuffer {
    fn drop(&mut self) {
        self.zeroize();
        #[cfg(test)]
        ZEROIZED_ON_DROP.with(|counter| counter.set(counter.get() + 1));
    }
}

impl fmt::Debug for FrameBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FrameBuffer")
            .field("len", &self.data.len())
            .field("zeroized", &self.is_zeroized())
            .finish()
    }
}

/// Content-free metadata for one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameMeta {
    /// Frame id.
    pub id: FrameId,
    /// Capture time relative to process start.
    pub captured_at: Duration,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// 32-byte correlation hash of the frame.
    pub hash: [u8; 32],
}

/// The result of asking a backend to lock a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlockStatus {
    /// The range is locked in RAM.
    Locked,
    /// The platform does not expose `mlock` to this build.
    Unsupported,
    /// The call was made and failed.
    Failed,
}

/// A best-effort page-locking backend.
pub trait MlockBackend: fmt::Debug + Send + Sync {
    /// Attempts to lock the pages holding `data`. Never fails the caller.
    ///
    /// The slice is the frame's own storage, so a real backend can pass
    /// `data.as_ptr()` and `data.len()` to `mlock(2)`.
    fn lock(&self, data: &[u8]) -> MlockStatus;

    /// Releases a previously locked range. `data` is the same storage the
    /// matching [`MlockBackend::lock`] call received.
    fn unlock(&self, data: &[u8]);
}

/// The default backend. Safe std-only Rust cannot call `mlock(2)`, so it always
/// reports [`MlockStatus::Unsupported`], which the ring treats as non-fatal.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoMlock;

impl MlockBackend for NoMlock {
    fn lock(&self, _data: &[u8]) -> MlockStatus {
        MlockStatus::Unsupported
    }

    fn unlock(&self, _data: &[u8]) {}
}

#[derive(Debug)]
struct Slot {
    frame: Option<FrameBuffer>,
    meta: Option<FrameMeta>,
    /// Bytes the backend actually locked for this frame. Zero when the lock
    /// was unsupported or failed, so eviction only unlocks what was locked.
    locked_bytes: usize,
}

/// Aggregate ring counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RingStats {
    /// Slot count.
    pub capacity: usize,
    /// Occupied slots.
    pub occupied: usize,
    /// Bytes currently held.
    pub resident_bytes: usize,
    /// Byte budget.
    pub budget_bytes: usize,
    /// Frames accepted.
    pub pushed: u64,
    /// Frames overwritten or evicted.
    pub overwritten: u64,
    /// Bytes the backend reports as locked.
    pub mlock_locked_bytes: usize,
}

/// A fixed-capacity ring of frames.
///
/// `head` is the slot index of the oldest occupied frame and `occupied` is how
/// many slots are live. A push writes at `(head + occupied) % capacity`, which is
/// always an empty slot unless the ring is full, in which case the oldest frame
/// is evicted first. This keeps the occupied region contiguous and makes
/// `latest` and `iter` O(1) to reason about.
pub struct RingBuffer {
    slots: Vec<Slot>,
    capacity: usize,
    budget_bytes: usize,
    head: usize,
    next_id: u64,
    occupied: usize,
    resident_bytes: usize,
    pushed: u64,
    overwritten: u64,
    mlock_locked_bytes: usize,
    mlock: Box<dyn MlockBackend>,
}

impl RingBuffer {
    /// Creates a ring with `capacity` slots and a byte budget.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::InvalidInput`] when `capacity` or
    /// `budget_bytes` is zero.
    pub fn new(capacity: usize, budget_bytes: usize, mlock: Box<dyn MlockBackend>) -> Result<Self> {
        if capacity == 0 {
            return Err(CurbCutError::InvalidInput {
                field: "capacity".to_string(),
            });
        }
        if budget_bytes == 0 {
            return Err(CurbCutError::InvalidInput {
                field: "budget_bytes".to_string(),
            });
        }
        Ok(Self {
            slots: (0..capacity)
                .map(|_| Slot {
                    frame: None,
                    meta: None,
                    locked_bytes: 0,
                })
                .collect(),
            capacity,
            budget_bytes,
            head: 0,
            next_id: 0,
            occupied: 0,
            resident_bytes: 0,
            pushed: 0,
            overwritten: 0,
            mlock_locked_bytes: 0,
            mlock,
        })
    }

    /// Creates a ring sized for `seconds` at `fps`.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::InvalidInput`] when `fps` is zero, when
    /// `fps * seconds` is zero or overflows, or when `budget_bytes` is zero.
    pub fn with_fps(
        fps: u32,
        seconds: u32,
        budget_bytes: usize,
        mlock: Box<dyn MlockBackend>,
    ) -> Result<Self> {
        if fps == 0 {
            return Err(CurbCutError::InvalidInput {
                field: "fps".to_string(),
            });
        }
        let capacity = (fps as u64)
            .checked_mul(u64::from(seconds))
            .and_then(|slots| usize::try_from(slots).ok())
            .ok_or_else(|| CurbCutError::InvalidInput {
                field: "fps * seconds".to_string(),
            })?;
        Self::new(capacity, budget_bytes, mlock)
    }

    /// Inserts a frame, evicting the oldest frames when the byte budget or the
    /// slot capacity requires it.
    ///
    /// The ring assigns the frame id; `meta.id` is ignored.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::InvalidInput`] when the frame is empty, larger
    /// than [`MAX_FRAME_BYTES`], or larger than the whole budget.
    pub fn push(&mut self, data: &[u8], meta: FrameMeta) -> Result<FrameId> {
        let len = data.len();
        if len == 0 {
            return Err(CurbCutError::InvalidInput {
                field: "frame is empty".to_string(),
            });
        }
        if len > MAX_FRAME_BYTES {
            return Err(CurbCutError::InvalidInput {
                field: format!("frame length exceeds {}", MAX_FRAME_BYTES),
            });
        }
        if len > self.budget_bytes {
            return Err(CurbCutError::InvalidInput {
                field: "frame exceeds the ring budget".to_string(),
            });
        }

        while self.occupied == self.capacity
            || (self.occupied > 0 && self.resident_bytes.saturating_add(len) > self.budget_bytes)
        {
            self.evict_oldest();
        }

        let index = (self.head + self.occupied) % self.capacity;
        debug_assert!(self.slots[index].frame.is_none());

        let mut frame = FrameBuffer::new(len)?;
        frame.as_mut_slice().copy_from_slice(data);
        let locked_bytes = match self.mlock.lock(frame.as_slice()) {
            MlockStatus::Locked => len,
            MlockStatus::Unsupported | MlockStatus::Failed => 0,
        };
        self.resident_bytes += len;
        self.mlock_locked_bytes += locked_bytes;

        let id = FrameId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        self.slots[index] = Slot {
            frame: Some(frame),
            meta: Some(FrameMeta { id, ..meta }),
            locked_bytes,
        };
        self.occupied += 1;
        self.pushed += 1;
        Ok(id)
    }

    /// Finds a frame by id.
    pub fn get(&self, id: FrameId) -> Option<(&[u8], &FrameMeta)> {
        self.slots
            .iter()
            .find_map(|slot| match (&slot.frame, &slot.meta) {
                (Some(frame), Some(meta)) if meta.id == id => Some((frame.as_slice(), meta)),
                _ => None,
            })
    }

    /// The most recently pushed frame.
    pub fn latest(&self) -> Option<(&[u8], &FrameMeta)> {
        if self.occupied == 0 {
            return None;
        }
        let index = (self.head + self.occupied - 1) % self.capacity;
        match (&self.slots[index].frame, &self.slots[index].meta) {
            (Some(frame), Some(meta)) => Some((frame.as_slice(), meta)),
            _ => None,
        }
    }

    /// Iterates occupied slots oldest first.
    pub fn iter(&self) -> impl Iterator<Item = (&[u8], &FrameMeta)> {
        (0..self.occupied).filter_map(move |offset| {
            let index = (self.head + offset) % self.capacity;
            match (&self.slots[index].frame, &self.slots[index].meta) {
                (Some(frame), Some(meta)) => Some((frame.as_slice(), meta)),
                _ => None,
            }
        })
    }

    /// Occupied slot count.
    pub fn len(&self) -> usize {
        self.occupied
    }

    /// Whether the ring is empty.
    pub fn is_empty(&self) -> bool {
        self.occupied == 0
    }

    /// Slot count.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Bytes currently held.
    pub fn resident_bytes(&self) -> usize {
        self.resident_bytes
    }

    /// Byte budget.
    pub fn budget_bytes(&self) -> usize {
        self.budget_bytes
    }

    /// Frames accepted.
    pub fn pushed(&self) -> u64 {
        self.pushed
    }

    /// Frames overwritten or evicted.
    pub fn overwritten(&self) -> u64 {
        self.overwritten
    }

    /// Bytes the backend reports as locked.
    pub fn mlock_locked_bytes(&self) -> usize {
        self.mlock_locked_bytes
    }

    /// Snapshot of the counters.
    pub fn stats(&self) -> RingStats {
        RingStats {
            capacity: self.capacity,
            occupied: self.occupied,
            resident_bytes: self.resident_bytes,
            budget_bytes: self.budget_bytes,
            pushed: self.pushed,
            overwritten: self.overwritten,
            mlock_locked_bytes: self.mlock_locked_bytes,
        }
    }

    fn evict_oldest(&mut self) {
        if self.occupied == 0 {
            return;
        }
        let index = self.head;
        self.clear_slot(index);
        self.head = (self.head + 1) % self.capacity;
        self.overwritten += 1;
    }

    fn clear_slot(&mut self, index: usize) {
        let taken = self.slots.get_mut(index).map(|slot| {
            let frame = slot.frame.take();
            let locked_bytes = std::mem::take(&mut slot.locked_bytes);
            (frame, locked_bytes)
        });
        if let Some((Some(mut frame), locked_bytes)) = taken {
            let len = frame.len();
            // Zeroize before unlock so the backend observes a scrubbed range,
            // and so the guard byte is in place even if unlock panics.
            frame.zeroize();
            if locked_bytes > 0 {
                self.mlock.unlock(frame.as_slice());
                self.mlock_locked_bytes = self.mlock_locked_bytes.saturating_sub(locked_bytes);
            }
            self.resident_bytes = self.resident_bytes.saturating_sub(len);
            self.occupied = self.occupied.saturating_sub(1);
        }
        if let Some(slot) = self.slots.get_mut(index) {
            slot.meta = None;
        }
    }
}

impl Drop for RingBuffer {
    fn drop(&mut self) {
        for index in 0..self.capacity {
            self.clear_slot(index);
        }
    }
}

impl fmt::Debug for RingBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RingBuffer")
            .field("capacity", &self.capacity)
            .field("occupied", &self.occupied)
            .field("resident_bytes", &self.resident_bytes)
            .field("budget_bytes", &self.budget_bytes)
            .field("pushed", &self.pushed)
            .field("overwritten", &self.overwritten)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Default)]
    struct CountingMlock {
        locked: AtomicUsize,
        failed: bool,
    }

    impl MlockBackend for CountingMlock {
        fn lock(&self, data: &[u8]) -> MlockStatus {
            if self.failed {
                return MlockStatus::Failed;
            }
            self.locked.fetch_add(data.len(), Ordering::Relaxed);
            MlockStatus::Locked
        }

        fn unlock(&self, data: &[u8]) {
            self.locked.fetch_sub(data.len(), Ordering::Relaxed);
        }
    }

    fn meta(id: u64) -> FrameMeta {
        FrameMeta {
            id: FrameId(id),
            captured_at: Duration::from_millis(id),
            width: 8,
            height: 8,
            hash: [0; 32],
        }
    }

    fn ring(capacity: usize, budget: usize) -> RingBuffer {
        RingBuffer::new(capacity, budget, Box::new(NoMlock)).expect("ring")
    }

    #[test]
    fn zeroize_writes_the_guard_byte() {
        let mut buffer = FrameBuffer::with_fill(1024, TEST_PATTERN);
        assert!(!buffer.is_zeroized());
        buffer.zeroize();
        assert!(buffer.is_zeroized());
        assert!(buffer.as_slice().iter().all(|byte| *byte == GUARD_BYTE));
    }

    #[test]
    fn drop_zeroizes_the_buffer() {
        ZEROIZED_ON_DROP.with(|counter| counter.set(0));
        {
            let _buffer = FrameBuffer::with_fill(64, TEST_PATTERN);
            assert_eq!(ZEROIZED_ON_DROP.with(|counter| counter.get()), 0);
        }
        assert_eq!(ZEROIZED_ON_DROP.with(|counter| counter.get()), 1);
    }

    #[test]
    fn an_empty_frame_is_rejected() {
        let mut ring = ring(4, 1024);
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
    fn an_oversized_frame_is_rejected() {
        let mut ring = ring(4, 1024);
        let err = ring
            .push(&vec![0u8; 2048], meta(1))
            .expect_err("over budget");
        assert!(err.to_string().contains("budget"), "{err}");
        assert!(ring.is_empty());
    }

    #[test]
    fn a_zero_capacity_ring_is_rejected() {
        assert!(RingBuffer::new(0, 1024, Box::new(NoMlock)).is_err());
    }

    #[test]
    fn the_ring_overwrites_the_oldest_frame() {
        let mut ring = ring(3, 1024);
        let first = ring.push(&[1, 1, 1], meta(1)).expect("push");
        ring.push(&[2, 2, 2], meta(2)).expect("push");
        ring.push(&[3, 3, 3], meta(3)).expect("push");
        assert_eq!(ring.len(), 3);
        ring.push(&[4, 4, 4], meta(4)).expect("push");
        assert_eq!(ring.len(), 3, "capacity is fixed");
        assert!(
            ring.get(first).is_none(),
            "the oldest frame was overwritten"
        );
        assert_eq!(ring.overwritten(), 1);
        assert_eq!(ring.latest().expect("latest").1.id, FrameId(3));
    }

    #[test]
    fn the_byte_budget_evicts_old_frames() {
        let mut ring = ring(8, 64);
        for id in 0..8 {
            ring.push(&[0u8; 16], meta(id)).expect("push");
        }
        assert_eq!(ring.resident_bytes(), 64, "the budget is respected");
        assert_eq!(ring.len(), 4, "only four 16-byte frames fit in 64 bytes");
        assert_eq!(ring.overwritten(), 4);
        assert!(ring.latest().is_some());
    }

    #[test]
    fn mlock_failure_is_non_fatal() {
        let backend = Box::new(CountingMlock {
            locked: AtomicUsize::new(0),
            failed: true,
        });
        let mut buffer = RingBuffer::new(4, 1024, backend).expect("ring");
        let id = buffer.push(&[9u8; 8], meta(1)).expect("push succeeds");
        assert_eq!(buffer.get(id).expect("frame").0, &[9u8; 8]);
        assert_eq!(buffer.mlock_locked_bytes(), 0);
    }

    #[test]
    fn a_locking_backend_is_tracked() {
        let backend = Box::new(CountingMlock {
            locked: AtomicUsize::new(0),
            failed: false,
        });
        let mut buffer = RingBuffer::new(4, 1024, backend).expect("ring");
        buffer.push(&[1u8; 10], meta(1)).expect("push");
        assert_eq!(buffer.mlock_locked_bytes(), 10);
    }

    #[test]
    fn memory_stays_within_budget_over_a_long_run() {
        // 30 minutes at 2 fps is 3600 frames.
        let mut ring =
            RingBuffer::with_fps(2, 30 * 60, 128 * 1024, Box::new(NoMlock)).expect("ring");
        assert_eq!(ring.capacity(), 3600);
        for id in 0..3600u64 {
            ring.push(&[0u8; 64], meta(id)).expect("push");
            assert!(
                ring.resident_bytes() <= ring.budget_bytes(),
                "budget exceeded at frame {id}"
            );
        }
        assert_eq!(ring.pushed(), 3600);
        // 128 KiB / 64 B = 2048 resident frames; the rest were evicted.
        assert_eq!(ring.len(), 2048);
        assert_eq!(ring.resident_bytes(), 2048 * 64);
        assert_eq!(ring.overwritten(), 3600 - 2048);
    }

    #[test]
    fn a_small_budget_bounds_a_long_run() {
        let mut ring = RingBuffer::with_fps(2, 30 * 60, 2048, Box::new(NoMlock)).expect("ring");
        for id in 0..3600u64 {
            ring.push(&[0u8; 64], meta(id)).expect("push");
            assert!(ring.resident_bytes() <= 2048);
        }
        assert_eq!(ring.len(), 2048 / 64);
        assert_eq!(ring.overwritten(), 3600 - 32);
    }

    #[test]
    fn iter_yields_oldest_first_after_wrap() {
        let mut ring = ring(3, 1024);
        for id in 0..5u64 {
            ring.push(&[id as u8; 1], meta(id)).expect("push");
        }
        let ids: Vec<FrameId> = ring.iter().map(|(_, frame_meta)| frame_meta.id).collect();
        assert_eq!(ids, vec![FrameId(2), FrameId(3), FrameId(4)]);
        assert_eq!(ring.latest().expect("latest").1.id, FrameId(4));
        assert!(ring.get(FrameId(0)).is_none());
        assert_eq!(ring.get(FrameId(2)).expect("frame").0, &[2u8; 1]);
    }

    #[test]
    fn debug_impls_print_no_pixels() {
        let mut ring = ring(2, 1024);
        ring.push(&[7u8; 16], meta(1)).expect("push");
        assert_eq!(
            format!("{ring:?}"),
            "RingBuffer { capacity: 2, occupied: 1, resident_bytes: 16, \
             budget_bytes: 1024, pushed: 1, overwritten: 0 }"
        );

        let mut buffer = FrameBuffer::with_fill(16, 7);
        assert_eq!(
            format!("{buffer:?}"),
            "FrameBuffer { len: 16, zeroized: false }"
        );
        buffer.zeroize();
        assert_eq!(
            format!("{buffer:?}"),
            "FrameBuffer { len: 16, zeroized: true }"
        );
    }

    #[derive(Debug, Default)]
    struct ProbeState {
        unlocks: AtomicUsize,
        dirty: AtomicUsize,
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
            self.state.unlocks.fetch_add(1, Ordering::Relaxed);
            if !data.iter().all(|byte| *byte == GUARD_BYTE) {
                self.state.dirty.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    #[test]
    fn eviction_unlocks_a_zeroized_range() {
        let probe = ZeroizeProbe::default();
        let state = probe.state.clone();
        let mut ring = RingBuffer::new(2, 1024, Box::new(probe)).expect("ring");
        for id in 0..4u64 {
            ring.push(&[id as u8; 8], meta(id)).expect("push");
        }
        assert_eq!(
            state.unlocks.load(Ordering::Relaxed),
            2,
            "two frames were evicted"
        );
        assert_eq!(
            state.dirty.load(Ordering::Relaxed),
            0,
            "every evicted range was scrubbed before unlock"
        );
    }
}
