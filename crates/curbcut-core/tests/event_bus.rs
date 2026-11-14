//! Integration tests for the event bus, exercised through the public API.

use std::sync::Arc;
use std::thread;

use curbcut_core::event::{EventBus, EventKind, EventSummary, OverflowPolicy, Producer};

#[test]
fn backpressure_is_per_subscriber() {
    let bus = EventBus::new();
    let slow = bus
        .subscribe("slow", 2, OverflowPolicy::DropNewest)
        .expect("valid subscription");
    let fast = bus
        .subscribe("fast", 64, OverflowPolicy::DropNewest)
        .expect("valid subscription");

    for _ in 0..20 {
        bus.publish(
            EventKind::FrameCaptured,
            Producer::Capture,
            EventSummary::new(),
        );
    }

    assert_eq!(fast.len(), 20, "a fast consumer must not lose events");
    assert_eq!(slow.len(), 2);
    assert_eq!(slow.dropped(), 18);
    assert_eq!(fast.dropped(), 0);

    let stats = bus.stats();
    assert_eq!(stats.published, 20);
    assert_eq!(stats.dropped, 18);
    assert_eq!(stats.subscribers, 2);
}

#[test]
fn critical_events_survive_a_slow_consumer() {
    let bus = EventBus::new();
    let slow = bus
        .subscribe("slow", 1, OverflowPolicy::DropNonCritical)
        .expect("valid subscription");

    for _ in 0..5 {
        bus.publish(
            EventKind::FrameCaptured,
            Producer::Capture,
            EventSummary::new(),
        );
    }
    bus.publish(
        EventKind::PanicRequested,
        Producer::Input,
        EventSummary::new(),
    );

    let events = slow.drain();
    assert!(events.iter().any(|e| e.kind == EventKind::PanicRequested));
    assert!(slow.dropped() >= 1);
}

#[test]
fn trace_id_propagates_across_tasks() {
    let bus = Arc::new(EventBus::new());
    let consumer = bus
        .subscribe("consumer", 16, OverflowPolicy::default())
        .expect("valid subscription");

    let (tx, rx) = std::sync::mpsc::channel();

    // Capture task: publishes a frame and hands its trace to the vision task.
    let capture_bus = Arc::clone(&bus);
    let capture = thread::spawn(move || {
        let trace = capture_bus.publish(
            EventKind::FrameCaptured,
            Producer::Capture,
            EventSummary::new(),
        );
        tx.send(trace).expect("send trace");
    });

    // Vision task: continues the same trace instead of starting a new one.
    let vision_bus = Arc::clone(&bus);
    let vision = thread::spawn(move || {
        let trace = rx.recv().expect("receive trace");
        vision_bus.publish_with_trace(
            trace,
            EventKind::ElementsDetected,
            Producer::Vision,
            EventSummary::new().with_count(2),
        );
    });

    capture.join().expect("capture thread");
    vision.join().expect("vision thread");

    let events = consumer.drain();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].trace_id, events[1].trace_id);
    assert_eq!(events[0].producer, Producer::Capture);
    assert_eq!(events[1].producer, Producer::Vision);
}

#[test]
fn dropped_events_are_surfaced_in_stats() {
    let bus = EventBus::new();
    let rx = bus
        .subscribe("rx", 1, OverflowPolicy::DropOldest)
        .expect("valid subscription");
    bus.publish(EventKind::GraphMerged, Producer::Merge, EventSummary::new());
    bus.publish(EventKind::GraphMerged, Producer::Merge, EventSummary::new());

    assert_eq!(rx.dropped(), 1);
    assert_eq!(bus.stats().dropped, 1);
    assert_eq!(bus.stats().delivered, 2);
}

#[test]
fn unsubscribe_removes_the_queue() {
    let bus = EventBus::new();
    let rx = bus
        .subscribe("temp", 4, OverflowPolicy::default())
        .expect("valid subscription");
    let id = rx.id();
    assert!(bus.unsubscribe(id));
    bus.publish(EventKind::GraphMerged, Producer::Merge, EventSummary::new());
    assert_eq!(rx.len(), 0);
    assert_eq!(bus.stats().subscribers, 0);
}
