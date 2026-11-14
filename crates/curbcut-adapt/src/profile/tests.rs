//! Tests for the profile graph.

use std::sync::{Arc, Mutex};

use super::*;
use crate::settings::{SettingKey, SettingValue, PROFILE_GRAPH_VERSION};
use curbcut_core::id::ProfileId;

/// A sink that records every event it is sent.
#[derive(Debug, Default, Clone)]
struct Recording {
    events: Arc<Mutex<Vec<PreviewEvent>>>,
}

impl Recording {
    fn events(&self) -> Vec<PreviewEvent> {
        self.events.lock().expect("the lock is uncontended").clone()
    }
}

impl PreviewSink for Recording {
    fn on_change(&self, event: &PreviewEvent) {
        self.events
            .lock()
            .expect("the lock is uncontended")
            .push(event.clone());
    }
}

#[test]
fn a_built_in_profile_starts_from_its_baseline() {
    let graph = ProfileGraph::new(ProfileId::focus());
    assert_eq!(graph.version(), PROFILE_GRAPH_VERSION);
    assert_eq!(graph.kind(), ProfileKind::Focus);
    assert_eq!(graph.name(), "Focus");
    assert!(!graph.description().is_empty());
    assert!(!graph.is_modified());
    assert!(graph.settings().dim.enabled);
    assert!(graph.settings().stop_motion);
    assert!(graph.is_active(), "focus is the default active profile");
}

#[test]
fn a_custom_id_starts_from_neutral_defaults() {
    let graph = ProfileGraph::new(ProfileId::custom("reading-plus"));
    assert_eq!(graph.kind(), ProfileKind::Custom);
    assert_eq!(graph.name(), "reading-plus");
    assert_eq!(graph.description(), "");
    assert!(!graph.is_modified());
    assert!(!graph.is_active());
    assert!(!graph.settings().dim.enabled);
    assert!(!graph.settings().stop_motion);
    assert_eq!(graph.get(SettingKey::DimOpacity), SettingValue::Float(0.75));
}

#[test]
fn every_built_in_kind_maps_to_its_id() {
    let cases = [
        (ProfileId::focus(), ProfileKind::Focus),
        (ProfileId::read(), ProfileKind::Read),
        (ProfileId::simple(), ProfileKind::Simple),
        (ProfileId::calm(), ProfileKind::Calm),
    ];
    for (id, kind) in cases {
        assert_eq!(ProfileKind::from_id(&id), kind);
        assert_eq!(kind.baseline_id(), Some(id.clone()));
        let graph = ProfileGraph::new(id);
        assert_eq!(graph.kind(), kind);
        assert_eq!(graph.name(), kind.default_name());
    }
    assert_eq!(ProfileKind::Custom.baseline_id(), None);
}

#[test]
fn kinds_parse_and_reject() {
    for kind in ProfileKind::ALL {
        assert_eq!(ProfileKind::parse(kind.as_str()), Ok(kind));
    }
    let error = ProfileKind::parse("unknown").expect_err("unknown kind");
    assert!(error.to_string().contains("profile.kind"), "{error}");
}

#[test]
fn an_apply_notifies_the_sinks_after_it_commits() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    let sink = Recording::default();
    graph.subscribe(Box::new(sink.clone()));
    assert_eq!(graph.sink_count(), 1);

    graph
        .apply(SettingKey::MotionStop, SettingValue::Bool(false))
        .expect("valid");

    let events = sink.events();
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].key, SettingKey::MotionStop);
    assert_eq!(events[0].before, SettingValue::Bool(true));
    assert_eq!(events[0].after, SettingValue::Bool(false));
    assert_eq!(events[0].source, ChangeSource::Apply);
    assert_eq!(events[0].profile, ProfileId::focus());
    assert!(graph.is_modified());
}

#[test]
fn a_failed_apply_changes_nothing_and_notifies_no_one() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    let sink = Recording::default();
    graph.subscribe(Box::new(sink.clone()));

    let error = graph
        .apply(SettingKey::IconGridGap, SettingValue::Int(65))
        .expect_err("out of range");
    assert!(error.to_string().contains("gap"), "{error}");
    assert!(sink.events().is_empty());
    assert!(!graph.is_modified());

    let error = graph
        .apply(SettingKey::ConfigVersion, SettingValue::Int(99))
        .expect_err("read-only");
    assert!(error.to_string().contains("read-only"), "{error}");
    assert!(sink.events().is_empty());
}

#[test]
fn apply_many_is_atomic() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    let sink = Recording::default();
    graph.subscribe(Box::new(sink.clone()));

    let changes = vec![
        (SettingKey::MotionStop, SettingValue::Bool(false)),
        (SettingKey::ReadingRuler, SettingValue::Bool(true)),
    ];
    let events = graph.apply_many(&changes).expect("valid");
    assert_eq!(events.len(), 2, "{events:?}");
    assert_eq!(sink.events().len(), 2);
    assert!(!graph.settings().stop_motion);
    assert!(graph.settings().ruler);

    let before = sink.events().len();
    let bad = vec![
        (SettingKey::MotionStop, SettingValue::Bool(true)),
        (SettingKey::IconGridGap, SettingValue::Int(500)),
    ];
    assert!(graph.apply_many(&bad).is_err());
    assert_eq!(
        sink.events().len(),
        before,
        "a failed batch must not notify"
    );
    assert!(
        !graph.settings().stop_motion,
        "the batch must not partially commit"
    );
    assert!(graph.settings().ruler);
}

#[test]
fn apply_many_does_not_depend_on_the_order_of_section_toggles() {
    // A nested key listed before the toggle that enables its section must work
    // as well as the reverse order.
    let changes = [
        (SettingKey::ReflowSizePt, SettingValue::Float(24.0)),
        (SettingKey::ReflowEnabled, SettingValue::Bool(true)),
    ];

    let mut graph = ProfileGraph::new(ProfileId::calm());
    graph.apply_many(&changes).expect("valid");

    assert!(graph.settings().reflow.is_some());
    assert_eq!(
        graph.get(SettingKey::ReflowEnabled),
        SettingValue::Bool(true)
    );
    assert_eq!(
        graph.get(SettingKey::ReflowSizePt),
        SettingValue::Float(24.0)
    );

    let mut reversed = ProfileGraph::new(ProfileId::calm());
    reversed
        .apply_many(&[
            (SettingKey::ReflowEnabled, SettingValue::Bool(true)),
            (SettingKey::ReflowSizePt, SettingValue::Float(24.0)),
        ])
        .expect("valid");
    assert_eq!(graph.data.settings, reversed.data.settings);
    assert_eq!(graph.id(), reversed.id());
}

#[test]
fn a_batch_that_disables_a_section_and_writes_into_it_is_rejected() {
    // This is the case that was order-dependent before `order_for_apply`: the
    // nested key was written first, which created the section, and the disable
    // then cleared it. Both orders must now fail and leave the graph alone.
    let mut graph = ProfileGraph::new(ProfileId::focus());
    let sink = Recording::default();
    graph.subscribe(Box::new(sink.clone()));
    assert!(graph.settings().icon_grid.is_some());
    assert!(!graph.settings().icon_grid.as_ref().expect("on").symbols);

    for changes in [
        vec![
            (SettingKey::IconGridSymbols, SettingValue::Bool(false)),
            (SettingKey::IconGridEnabled, SettingValue::Bool(false)),
        ],
        vec![
            (SettingKey::IconGridEnabled, SettingValue::Bool(false)),
            (SettingKey::IconGridSymbols, SettingValue::Bool(false)),
        ],
    ] {
        let error = graph
            .apply_many(&changes)
            .expect_err("the section ends off");
        let message = error.to_string();
        assert!(message.contains("requires"), "{message}");
        assert!(
            graph.settings().icon_grid.is_some(),
            "a rejected batch must not disable the section"
        );
        assert!(
            !graph.settings().icon_grid.as_ref().expect("on").symbols,
            "a rejected batch must not write the nested key"
        );
    }
    assert!(sink.events().is_empty());
    assert!(!graph.is_modified());
}

#[test]
fn apply_many_deduplicates_keys_and_reports_the_final_value() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    let sink = Recording::default();
    graph.subscribe(Box::new(sink.clone()));

    // The last write wins, so the final value equals the baseline: no change,
    // and therefore no event. Computing `before` from the pre-batch graph and
    // the requested value would have emitted a false `true -> false`.
    let events = graph
        .apply_many(&[
            (SettingKey::MotionStop, SettingValue::Bool(false)),
            (SettingKey::MotionStop, SettingValue::Bool(true)),
        ])
        .expect("valid");
    assert!(events.is_empty(), "{events:?}");
    assert!(sink.events().is_empty());
    assert!(graph.settings().stop_motion);

    let events = graph
        .apply_many(&[
            (SettingKey::MotionStop, SettingValue::Bool(true)),
            (SettingKey::MotionStop, SettingValue::Bool(false)),
        ])
        .expect("valid");
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].key, SettingKey::MotionStop);
    assert_eq!(events[0].before, SettingValue::Bool(true));
    assert_eq!(events[0].after, SettingValue::Bool(false));
    assert!(!graph.settings().stop_motion);
}

#[test]
fn a_no_op_apply_emits_no_event() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    let sink = Recording::default();
    graph.subscribe(Box::new(sink.clone()));

    graph
        .apply(SettingKey::MotionStop, SettingValue::Bool(true))
        .expect("valid");
    assert!(
        sink.events().is_empty(),
        "writing the value already stored is not a change"
    );
    assert!(!graph.is_modified());

    graph
        .apply(SettingKey::MotionStop, SettingValue::Bool(false))
        .expect("valid");
    assert_eq!(sink.events().len(), 1);
}

#[test]
fn an_apply_event_reports_the_stored_value() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    let sink = Recording::default();
    graph.subscribe(Box::new(sink.clone()));

    graph
        .apply(SettingKey::DimOpacity, SettingValue::Float(0.4))
        .expect("valid");

    let events = sink.events();
    assert_eq!(events.len(), 1);
    // The core stores `f32`; `after` is what `get` returns, not the `f64` the
    // caller asked for.
    assert_eq!(events[0].after, graph.get(SettingKey::DimOpacity));
    assert_eq!(events[0].after, SettingValue::Float(0.4f32 as f64));
}

#[test]
fn apply_many_is_atomic_for_read_only_and_semantic_failures() {
    // Read-only key in the middle of an otherwise valid batch.
    let mut graph = ProfileGraph::new(ProfileId::focus());
    let sink = Recording::default();
    graph.subscribe(Box::new(sink.clone()));

    let error = graph
        .apply_many(&[
            (SettingKey::MotionStop, SettingValue::Bool(false)),
            (SettingKey::ReadingGrade, SettingValue::Int(4)),
        ])
        .expect_err("read-only");
    assert!(error.to_string().contains("read-only"), "{error}");
    assert!(graph.settings().stop_motion, "the batch must not commit");
    assert!(sink.events().is_empty());

    // Semantic failure: an unknown panic key makes the graph invalid even
    // though the value's own type is fine.
    let error = graph
        .apply_many(&[
            (SettingKey::MotionStop, SettingValue::Bool(false)),
            (
                SettingKey::ConfigPanicKey,
                SettingValue::KeyList(vec!["not-a-key".to_string()]),
            ),
        ])
        .expect_err("unknown panic key");
    assert!(error.to_string().contains("panic_key"), "{error}");
    assert!(graph.settings().stop_motion, "the batch must not commit");
    assert!(sink.events().is_empty());
}

#[test]
fn reset_preserves_activation() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    graph.set_active(false);
    graph
        .apply(SettingKey::MotionStop, SettingValue::Bool(false))
        .expect("valid");

    graph.reset();

    assert!(!graph.is_active(), "activation is document state");
    assert!(graph.settings().stop_motion);
}

#[test]
fn reset_restores_the_whole_baseline() {
    let mut graph = ProfileGraph::new(ProfileId::calm());
    let sink = Recording::default();
    graph.subscribe(Box::new(sink.clone()));

    graph
        .apply_many(&[
            (
                SettingKey::ProfileName,
                SettingValue::Text("Temporary".to_string()),
            ),
            (
                SettingKey::ProfileDescription,
                SettingValue::Text(String::new()),
            ),
            (SettingKey::ConfigHotReload, SettingValue::Bool(false)),
            (SettingKey::ReadingRuler, SettingValue::Bool(true)),
        ])
        .expect("valid");
    assert_eq!(graph.name(), "Temporary");
    assert!(graph.is_modified());

    let before = sink.events().len();
    let events = graph.reset();

    assert_eq!(graph.name(), "Calm");
    assert_eq!(graph.description(), ProfileKind::Calm.default_description());
    assert_eq!(
        graph.get(SettingKey::ConfigHotReload),
        SettingValue::Bool(true)
    );
    assert!(!graph.settings().ruler);
    assert!(!graph.is_modified());
    for key in [
        SettingKey::ProfileName,
        SettingKey::ProfileDescription,
        SettingKey::ConfigHotReload,
        SettingKey::ReadingRuler,
    ] {
        assert!(
            events.iter().any(|event| event.key == key),
            "{key:?} was not reported: {events:?}"
        );
    }
    assert!(sink.events().len() == before + events.len());
}

#[test]
fn changed_keys_lists_only_the_settings_that_differ() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    assert!(graph.changed_keys().is_empty());

    graph
        .apply(SettingKey::MotionStop, SettingValue::Bool(false))
        .expect("valid");

    let keys = graph.changed_keys();
    assert!(keys.contains(&SettingKey::MotionStop), "{keys:?}");
    assert!(keys.contains(&SettingKey::ProfileModified), "{keys:?}");
}

#[test]
fn copy_creates_a_custom_profile_with_the_copied_baseline() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    graph
        .apply(SettingKey::MotionStop, SettingValue::Bool(false))
        .expect("valid");

    let copied = graph.copy("reading-plus").expect("valid slug");
    assert_eq!(copied.id(), &ProfileId::custom("reading-plus"));
    assert_eq!(copied.kind(), ProfileKind::Custom);
    assert_eq!(copied.name(), "Focus copy");
    assert!(!copied.is_active());
    assert!(
        !copied.settings().stop_motion,
        "the copy keeps the source settings"
    );
    assert!(
        !copied.is_modified(),
        "the copy's baseline is the copied state"
    );
    assert!(copied.changed_keys().is_empty());

    let mut copied = copied;
    copied
        .apply(SettingKey::ReadingRuler, SettingValue::Bool(true))
        .expect("valid");
    assert!(copied.is_modified());
    copied.reset();
    assert!(!copied.settings().ruler, "reset returns to the copy");
    assert!(!copied.settings().stop_motion);
}

#[test]
fn copy_rejects_a_bad_slug() {
    let graph = ProfileGraph::new(ProfileId::focus());
    for slug in ["", "has space", "slash/name"] {
        let error = graph.copy(slug).expect_err("invalid slug");
        assert!(error.to_string().contains("profile.id"), "{slug:?}");
    }
    let long = "x".repeat(65);
    let error = graph.copy(&long).expect_err("too long");
    assert!(error.to_string().contains("profile.id"), "{error}");
}

#[test]
fn activation_is_not_a_setting_and_emits_no_event() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    let sink = Recording::default();
    graph.subscribe(Box::new(sink.clone()));

    graph.set_active(false);

    assert!(!graph.is_active());
    assert!(sink.events().is_empty());
    assert!(!graph.is_modified(), "activation is not a profile change");
}

#[test]
fn the_graph_validates_its_panic_key() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    let error = graph
        .apply(
            SettingKey::ConfigPanicKey,
            SettingValue::KeyList(vec!["not-a-key".to_string()]),
        )
        .expect_err("unknown key");
    assert!(error.to_string().contains("panic_key"), "{error}");
}

#[test]
fn the_graph_rejects_an_empty_display_name() {
    // `ProfileId::custom("")` is constructible, so `new` can produce an empty
    // name; `validate` is the invariant that catches it.
    let graph = ProfileGraph::new(ProfileId::custom(""));
    let error = graph.validate().expect_err("an empty name is invalid");
    assert!(error.to_string().contains("profile.name"), "{error}");

    let mut graph = ProfileGraph::new(ProfileId::focus());
    let error = graph
        .apply(
            SettingKey::ProfileName,
            SettingValue::Text("   ".to_string()),
        )
        .expect_err("whitespace is empty");
    assert!(error.to_string().contains("profile.name"), "{error}");
    assert_eq!(graph.name(), "Focus", "the failed write must not land");
}

#[test]
fn the_graph_reports_its_schema_version() {
    let graph = ProfileGraph::new(ProfileId::focus());
    assert_eq!(graph.version(), PROFILE_GRAPH_VERSION);
    assert_eq!(
        graph.get(SettingKey::ConfigVersion),
        SettingValue::Int(i64::from(PROFILE_GRAPH_VERSION))
    );
}

#[test]
fn every_writable_key_round_trips_through_the_graph() {
    let mut graph = ProfileGraph::new(ProfileId::calm());
    // Floats are values exactly representable in `f32`, which is how the core
    // stores them; `float_settings_are_stored_at_f32_precision` covers the
    // quantization of an arbitrary value.
    let cases = [
        (SettingKey::DimEnabled, SettingValue::Bool(true)),
        (SettingKey::DimOpacity, SettingValue::Float(0.5)),
        (
            SettingKey::DimExcept,
            SettingValue::TextList(vec!["sidebar".to_string()]),
        ),
        (SettingKey::MotionStop, SettingValue::Bool(false)),
        (
            SettingKey::MotionReduceTransparency,
            SettingValue::Bool(true),
        ),
        (
            SettingKey::InterruptionsMode,
            SettingValue::Text("block".to_string()),
        ),
        (SettingKey::IconGridEnabled, SettingValue::Bool(true)),
        (SettingKey::IconGridMinTile, SettingValue::Int(88)),
        (SettingKey::IconGridColumns, SettingValue::Int(2)),
        (SettingKey::IconGridSymbols, SettingValue::Bool(true)),
        (SettingKey::IconGridGap, SettingValue::Int(24)),
        (SettingKey::PathsShort, SettingValue::Bool(true)),
        (
            SettingKey::ReadingLevel,
            SettingValue::Text("grade-6".to_string()),
        ),
        (SettingKey::ReadingRuler, SettingValue::Bool(true)),
        (SettingKey::SpeechRate, SettingValue::Float(1.5)),
        (
            SettingKey::SpeechVoice,
            SettingValue::Text("custom-voice".to_string()),
        ),
        (SettingKey::ReflowEnabled, SettingValue::Bool(true)),
        (
            SettingKey::ReflowFont,
            SettingValue::Text("Atkinson Hyperlegible".to_string()),
        ),
        (SettingKey::ReflowSizePt, SettingValue::Float(24.0)),
        (SettingKey::ReflowLineHeight, SettingValue::Float(1.5)),
        (
            SettingKey::ReflowAlign,
            SettingValue::Text("center".to_string()),
        ),
        (SettingKey::ReflowLineLength, SettingValue::Int(42)),
        (SettingKey::ReflowWordSpacing, SettingValue::Float(1.5)),
        (SettingKey::ReflowLetterSpacing, SettingValue::Float(1.25)),
        (SettingKey::ReadAloudEnabled, SettingValue::Bool(true)),
        (
            SettingKey::ReadAloudHighlight,
            SettingValue::Text("line".to_string()),
        ),
        (SettingKey::ReadAloudRate, SettingValue::Float(0.75)),
        (SettingKey::StepsOne, SettingValue::Bool(true)),
        (SettingKey::StepsMemoryAids, SettingValue::Bool(true)),
        (SettingKey::StepsConfirmActions, SettingValue::Bool(true)),
        (SettingKey::ConfigHotReload, SettingValue::Bool(false)),
        (
            SettingKey::ConfigPanicKey,
            SettingValue::KeyList(vec![
                "ctrl".to_string(),
                "shift".to_string(),
                "esc".to_string(),
            ]),
        ),
        (
            SettingKey::ProfileName,
            SettingValue::Text("Calm copy".to_string()),
        ),
        (
            SettingKey::ProfileDescription,
            SettingValue::Text("For evenings.".to_string()),
        ),
    ];

    assert_eq!(cases.len(), 34, "40 keys minus the 6 read-only ones");

    for (key, value) in cases {
        graph.apply(key, value.clone()).expect("valid");
        assert_eq!(graph.get(key), value, "{}", key.as_str());
    }
}

#[test]
fn float_settings_are_stored_at_f32_precision() {
    let mut graph = ProfileGraph::new(ProfileId::calm());
    graph
        .apply(SettingKey::DimOpacity, SettingValue::Float(0.4))
        .expect("valid");
    // The core stores `f32`, so a value read back is the `f32` nearest to the
    // requested `f64`. Callers compare with a tolerance, not exact equality.
    assert_eq!(
        graph.get(SettingKey::DimOpacity),
        SettingValue::Float(0.4f32 as f64)
    );
}
