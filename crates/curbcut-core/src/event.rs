//! Typed in-process event bus with per-consumer backpressure.
//!
//! Every event carries a [`TraceId`] so a frame can be followed across tasks
//! without carrying its content. Events hold counts and hashes only.
//!
//! Backpressure is per subscriber: a slow consumer drops according to its
//! [`OverflowPolicy`] while a fast consumer is unaffected. By default a slow
//! consumer drops non-critical events and keeps critical ones (profile
//! changes, permission changes, panic).

use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::error::{CurbCutError, Result};
use crate::id::{FrameId, ProfileId, TraceId};

/// Which pipeline produced an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Producer {
    /// Screen capture.
    Capture,
    /// Accessibility tree adapter.
    A11y,
    /// Vision pipeline.
    Vision,
    /// Graph merge.
    Merge,
    /// Understanding pipeline.
    Understand,
    /// Adaptation pipeline.
    Adapt,
    /// Overlay renderer.
    Render,
    /// Input router.
    Input,
    /// Text to speech.
    Tts,
    /// Core orchestration.
    Core,
    /// Local store.
    Store,
}

impl Producer {
    /// The stable string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Capture => "capture",
            Self::A11y => "a11y",
            Self::Vision => "vision",
            Self::Merge => "merge",
            Self::Understand => "understand",
            Self::Adapt => "adapt",
            Self::Render => "render",
            Self::Input => "input",
            Self::Tts => "tts",
            Self::Core => "core",
            Self::Store => "store",
        }
    }
}

impl fmt::Display for Producer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The event kinds from `docs/ARCHITECTURE.md` section 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EventKind {
    /// A frame was captured. The only event that may reference pixels, and it
    /// stays in-process.
    FrameCaptured,
    /// The accessibility tree changed.
    A11yTreeChanged,
    /// Vision detected elements.
    ElementsDetected,
    /// OCR produced text.
    TextRecognized,
    /// The screen graph was merged.
    GraphMerged,
    /// Content was analyzed.
    ContentAnalyzed,
    /// The active profile changed. Critical.
    ProfileChanged,
    /// An overlay plan was built.
    PlanBuilt,
    /// The overlay requested an action.
    OverlayAction,
    /// Speech was requested.
    SpeechRequested,
    /// Speech finished.
    SpeechFinished,
    /// The frontmost application changed.
    AppChanged,
    /// A capability grant or revoke happened. Critical.
    PermissionChanged,
    /// The user asked for panic. Critical.
    PanicRequested,
}

impl EventKind {
    /// Whether the event must survive backpressure.
    ///
    /// Critical events are profile changes, permission changes, and panic.
    pub fn is_critical(self) -> bool {
        matches!(
            self,
            Self::ProfileChanged | Self::PermissionChanged | Self::PanicRequested
        )
    }

    /// The stable string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FrameCaptured => "frame_captured",
            Self::A11yTreeChanged => "a11y_tree_changed",
            Self::ElementsDetected => "elements_detected",
            Self::TextRecognized => "text_recognized",
            Self::GraphMerged => "graph_merged",
            Self::ContentAnalyzed => "content_analyzed",
            Self::ProfileChanged => "profile_changed",
            Self::PlanBuilt => "plan_built",
            Self::OverlayAction => "overlay_action",
            Self::SpeechRequested => "speech_requested",
            Self::SpeechFinished => "speech_finished",
            Self::AppChanged => "app_changed",
            Self::PermissionChanged => "permission_changed",
            Self::PanicRequested => "panic_requested",
        }
    }
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A content-free summary of an event's payload.
///
/// Counts and hashes only: never text, labels, or pixels.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventSummary {
    /// How many elements, words, or regions are involved.
    pub count: usize,
    /// A 32-byte correlation hash over the content, if any.
    pub hash: Option<[u8; 32]>,
    /// The frame this event relates to.
    pub frame: Option<FrameId>,
    /// The profile this event relates to.
    pub profile: Option<ProfileId>,
}

impl EventSummary {
    /// An empty summary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the count.
    pub fn with_count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    /// Sets the correlation hash.
    pub fn with_hash(mut self, hash: [u8; 32]) -> Self {
        self.hash = Some(hash);
        self
    }

    /// Sets the related frame.
    pub fn with_frame(mut self, frame: FrameId) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Sets the related profile.
    pub fn with_profile(mut self, profile: ProfileId) -> Self {
        self.profile = Some(profile);
        self
    }
}

/// A typed event. It carries no content, only a trace id and a summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Correlates this event with every other event from the same cause.
    pub trace_id: TraceId,
    /// Monotonic bus sequence number.
    pub seq: u64,
    /// What happened.
    pub kind: EventKind,
    /// Who produced it.
    pub producer: Producer,
    /// Content-free payload summary.
    pub summary: EventSummary,
}

impl Event {
    /// Whether this event is critical and must survive backpressure.
    pub fn is_critical(&self) -> bool {
        self.kind.is_critical()
    }
}

/// What a subscriber does when its queue is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverflowPolicy {
    /// Drop the oldest queued event, always.
    DropOldest,
    /// Drop the incoming event, always.
    DropNewest,
    /// Drop the oldest non-critical event. If every queued event is critical,
    /// drop the incoming event.
    #[default]
    DropNonCritical,
}

/// Identifies one subscriber.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SubscriberId(pub u64);

/// Per-subscriber queue and drop counter.
#[derive(Debug, Default)]
struct SubscriberState {
    queue: VecDeque<Event>,
    dropped: u64,
}

/// A handle to one subscriber's queue.
///
/// Cloning a receiver creates another handle to the same queue; it does not
/// create another subscription.
#[derive(Debug, Clone)]
pub struct EventReceiver {
    id: SubscriberId,
    name: String,
    capacity: usize,
    policy: OverflowPolicy,
    state: Arc<Mutex<SubscriberState>>,
}

impl EventReceiver {
    /// The subscriber id.
    pub fn id(&self) -> SubscriberId {
        self.id
    }

    /// The subscriber name, for logs.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The queue capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// The overflow policy.
    pub fn policy(&self) -> OverflowPolicy {
        self.policy
    }

    /// Queued event count.
    pub fn len(&self) -> usize {
        self.lock().queue.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Events dropped for this subscriber.
    pub fn dropped(&self) -> u64 {
        self.lock().dropped
    }

    /// Takes the oldest event, if any.
    pub fn try_recv(&self) -> Option<Event> {
        self.lock().queue.pop_front()
    }

    /// Takes every queued event, oldest first.
    pub fn drain(&self) -> Vec<Event> {
        let mut state = self.lock();
        state.queue.drain(..).collect()
    }

    fn lock(&self) -> MutexGuard<'_, SubscriberState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Enqueues an event, applying the overflow policy.
    ///
    /// Returns `true` when the event was queued.
    fn push(&self, event: Event) -> bool {
        let mut state = self.lock();
        if state.queue.len() >= self.capacity {
            match self.policy {
                OverflowPolicy::DropNewest => {
                    state.dropped += 1;
                    return false;
                }
                OverflowPolicy::DropOldest => {
                    state.queue.pop_front();
                    state.dropped += 1;
                }
                OverflowPolicy::DropNonCritical => {
                    match state.queue.iter().position(|queued| !queued.is_critical()) {
                        Some(index) => {
                            state.queue.remove(index);
                            state.dropped += 1;
                        }
                        None => {
                            state.dropped += 1;
                            return false;
                        }
                    }
                }
            }
        }
        state.queue.push_back(event);
        true
    }
}

/// Aggregate bus counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BusStats {
    /// Events published.
    pub published: u64,
    /// Events enqueued into a subscriber.
    pub delivered: u64,
    /// Events dropped by subscriber queues.
    pub dropped: u64,
    /// Live subscribers.
    pub subscribers: usize,
}

#[derive(Debug, Default)]
struct BusInner {
    subscribers: Vec<Arc<EventReceiver>>,
}

/// The in-process event bus.
#[derive(Debug, Default)]
pub struct EventBus {
    inner: Mutex<BusInner>,
    trace_seq: AtomicU64,
    subscriber_seq: AtomicU64,
    seq: AtomicU64,
    published: AtomicU64,
    delivered: AtomicU64,
}

impl EventBus {
    /// Creates an empty bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribes a consumer with a bounded queue and an overflow policy.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::InvalidInput`] when `capacity` is zero.
    pub fn subscribe(
        &self,
        name: impl Into<String>,
        capacity: usize,
        policy: OverflowPolicy,
    ) -> Result<EventReceiver> {
        if capacity == 0 {
            return Err(CurbCutError::InvalidInput {
                field: "capacity".to_string(),
            });
        }
        let id = SubscriberId(self.subscriber_seq.fetch_add(1, Ordering::Relaxed));
        let receiver = EventReceiver {
            id,
            name: name.into(),
            capacity,
            policy,
            state: Arc::new(Mutex::new(SubscriberState::default())),
        };
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.subscribers.push(Arc::new(receiver.clone()));
        Ok(receiver)
    }

    /// Removes a subscriber. Returns whether it existed.
    pub fn unsubscribe(&self, id: SubscriberId) -> bool {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let before = inner.subscribers.len();
        inner.subscribers.retain(|sub| sub.id() != id);
        inner.subscribers.len() != before
    }

    /// Publishes an event with a fresh [`TraceId`] and returns it.
    pub fn publish(&self, kind: EventKind, producer: Producer, summary: EventSummary) -> TraceId {
        let trace_id = TraceId(self.trace_seq.fetch_add(1, Ordering::Relaxed));
        self.publish_with_trace(trace_id, kind, producer, summary)
    }

    /// Publishes an event that continues an existing trace.
    ///
    /// Use this when a downstream task reacts to an event, so the whole chain
    /// shares one [`TraceId`].
    pub fn publish_with_trace(
        &self,
        trace_id: TraceId,
        kind: EventKind,
        producer: Producer,
        summary: EventSummary,
    ) -> TraceId {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let event = Event {
            trace_id,
            seq,
            kind,
            producer,
            summary,
        };
        self.published.fetch_add(1, Ordering::Relaxed);
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        for subscriber in &inner.subscribers {
            if subscriber.push(event.clone()) {
                self.delivered.fetch_add(1, Ordering::Relaxed);
            }
        }
        trace_id
    }

    /// Snapshot of the aggregate counters.
    pub fn stats(&self) -> BusStats {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        BusStats {
            published: self.published.load(Ordering::Relaxed),
            delivered: self.delivered.load(Ordering::Relaxed),
            dropped: inner.subscribers.iter().map(|sub| sub.dropped()).sum(),
            subscribers: inner.subscribers.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary() -> EventSummary {
        EventSummary::new().with_count(3)
    }

    #[test]
    fn publish_delivers_to_every_subscriber() {
        let bus = EventBus::new();
        let a = bus.subscribe("a", 8, OverflowPolicy::default()).unwrap();
        let b = bus.subscribe("b", 8, OverflowPolicy::default()).unwrap();
        let trace = bus.publish(EventKind::GraphMerged, Producer::Merge, summary());
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 1);
        assert_eq!(a.try_recv().unwrap().trace_id, trace);
        assert_eq!(b.try_recv().unwrap().trace_id, trace);
        assert_eq!(bus.stats().published, 1);
        assert_eq!(bus.stats().delivered, 2);
    }

    #[test]
    fn trace_id_propagates_to_linked_events() {
        let bus = EventBus::new();
        let rx = bus.subscribe("rx", 8, OverflowPolicy::default()).unwrap();
        let first = bus.publish(EventKind::FrameCaptured, Producer::Capture, summary());
        let second = bus.publish_with_trace(
            first,
            EventKind::ElementsDetected,
            Producer::Vision,
            summary(),
        );
        assert_eq!(first, second);
        let events = rx.drain();
        assert_eq!(events[0].trace_id, events[1].trace_id);
    }

    #[test]
    fn slow_consumer_drops_non_critical_and_keeps_critical() {
        let bus = EventBus::new();
        let rx = bus
            .subscribe("slow", 2, OverflowPolicy::DropNonCritical)
            .unwrap();
        bus.publish(EventKind::FrameCaptured, Producer::Capture, summary());
        bus.publish(EventKind::TextRecognized, Producer::Vision, summary());
        bus.publish(
            EventKind::ProfileChanged,
            Producer::Render,
            summary().with_profile(ProfileId::focus()),
        );
        assert_eq!(rx.len(), 2);
        let events = rx.drain();
        assert!(events.iter().any(|e| e.kind == EventKind::ProfileChanged));
        assert!(rx.dropped() >= 1);
        assert!(bus.stats().dropped >= 1);
    }

    #[test]
    fn drop_oldest_keeps_the_newest() {
        let bus = EventBus::new();
        let rx = bus
            .subscribe("oldest", 2, OverflowPolicy::DropOldest)
            .unwrap();
        bus.publish(EventKind::FrameCaptured, Producer::Capture, summary());
        bus.publish(EventKind::FrameCaptured, Producer::Capture, summary());
        bus.publish(EventKind::FrameCaptured, Producer::Capture, summary());
        let events = rx.drain();
        assert_eq!(events.len(), 2);
        assert!(events[0].seq > 0);
        assert_eq!(rx.dropped(), 1);
    }

    #[test]
    fn drop_newest_keeps_the_oldest() {
        let bus = EventBus::new();
        let rx = bus
            .subscribe("newest", 1, OverflowPolicy::DropNewest)
            .unwrap();
        let first = bus.publish(EventKind::FrameCaptured, Producer::Capture, summary());
        bus.publish(EventKind::FrameCaptured, Producer::Capture, summary());
        let events = rx.drain();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].trace_id, first);
        assert_eq!(rx.dropped(), 1);
    }

    #[test]
    fn a_fast_consumer_is_unaffected_by_a_slow_one() {
        let bus = EventBus::new();
        let slow = bus
            .subscribe("slow", 1, OverflowPolicy::DropNewest)
            .unwrap();
        let fast = bus
            .subscribe("fast", 16, OverflowPolicy::default())
            .unwrap();
        for _ in 0..10 {
            bus.publish(EventKind::FrameCaptured, Producer::Capture, summary());
        }
        assert_eq!(slow.len(), 1);
        assert_eq!(fast.len(), 10);
        assert_eq!(slow.dropped(), 9);
        assert_eq!(fast.dropped(), 0);
    }

    #[test]
    fn zero_capacity_is_rejected() {
        let bus = EventBus::new();
        assert!(bus.subscribe("bad", 0, OverflowPolicy::default()).is_err());
    }

    #[test]
    fn unsubscribe_stops_delivery() {
        let bus = EventBus::new();
        let rx = bus.subscribe("temp", 4, OverflowPolicy::default()).unwrap();
        let id = rx.id();
        assert!(bus.unsubscribe(id));
        assert!(!bus.unsubscribe(id));
        bus.publish(EventKind::GraphMerged, Producer::Merge, summary());
        assert_eq!(rx.len(), 0);
    }

    #[test]
    fn critical_kinds_are_exactly_the_three_expected() {
        assert!(EventKind::ProfileChanged.is_critical());
        assert!(EventKind::PermissionChanged.is_critical());
        assert!(EventKind::PanicRequested.is_critical());
        assert!(!EventKind::FrameCaptured.is_critical());
        assert!(!EventKind::GraphMerged.is_critical());
    }

    #[test]
    fn summary_carries_counts_and_hashes_only() {
        let summary = EventSummary::new()
            .with_count(5)
            .with_hash([7; 32])
            .with_frame(FrameId(2))
            .with_profile(ProfileId::read());
        let dbg = format!("{summary:?}");
        assert!(dbg.contains("count: 5"));
        assert!(!dbg.contains("SECRET"));
    }
}
