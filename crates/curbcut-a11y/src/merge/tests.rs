//! Tests for the a11y + vision merge.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use curbcut_core::id::{AppId, ElementId};
use curbcut_core::types::{Element, ElementState, Role, Source};

use super::*;

fn app() -> AppId {
    AppId::new("demo")
}

fn element(id: u64, x: i32, y: i32, w: u32, h: u32, role: Role) -> Element {
    Element::new(ElementId(id), Rect::new(x, y, w, h), role, app())
}

fn detection(x: i32, y: i32, w: u32, h: u32, role: Role) -> VisionDetection {
    VisionDetection::new(Rect::new(x, y, w, h), role, 0.8)
}

#[test]
fn the_config_has_sane_defaults() {
    let config = MergeConfig::default();
    assert_eq!(config.min_iou(), DEFAULT_MIN_IOU);
    assert_eq!(config.rect_conflict_iou(), DEFAULT_RECT_CONFLICT_IOU);
    assert_eq!(config.vision_id_start(), DEFAULT_VISION_ID_START);
    assert_eq!(DEFAULT_MIN_IOU, 0.3);
    assert_eq!(DEFAULT_RECT_CONFLICT_IOU, 0.5);

    let clamped = MergeConfig::new(2.0, -1.0, 7);
    assert_eq!(clamped.min_iou(), 1.0);
    assert_eq!(clamped.rect_conflict_iou(), 0.0);
    assert_eq!(clamped.vision_id_start(), 7);
}

#[test]
fn role_compatibility_is_family_based() {
    assert_eq!(
        role_compatibility(&Role::Button, &Role::Button),
        RoleCompat::Same
    );
    assert_eq!(
        role_compatibility(&Role::Button, &Role::Link),
        RoleCompat::Compatible
    );
    assert_eq!(
        role_compatibility(&Role::StaticText, &Role::Heading),
        RoleCompat::Compatible
    );
    assert_eq!(
        role_compatibility(&Role::Button, &Role::StaticText),
        RoleCompat::Different
    );
    assert_eq!(
        role_compatibility(&Role::Image, &Role::Button),
        RoleCompat::Different
    );
    assert_eq!(
        role_compatibility(&Role::Unknown("weird".to_string()), &Role::Button),
        RoleCompat::Compatible
    );
}

#[test]
fn an_identical_pair_merges_into_one_element() {
    let engine = MergeEngine::default();
    let mut a = element(1, 0, 0, 100, 50, Role::Button);
    a.label = Some("Save".to_string());
    a.text = Some("Save".to_string());
    let v = detection(0, 0, 100, 50, Role::Button);

    let report = engine.merge(app(), &[a.clone()], std::slice::from_ref(&v));

    assert_eq!(report.len(), 1);
    assert_eq!(report.matched(), 1);
    assert_eq!(report.a11y_only(), 0);
    assert_eq!(report.vision_only(), 0);
    assert_eq!(report.conflicts(), 0);

    let merged = &report.elements()[0];
    assert!(merged.is_matched());
    assert!(!merged.is_semantic_only());
    assert!(!merged.is_vision_only());
    assert!(!merged.has_conflicts());

    let element = merged.element();
    assert_eq!(element.id, ElementId(1));
    assert_eq!(element.rect, Rect::new(0, 0, 100, 50));
    assert_eq!(element.role, Role::Button);
    assert_eq!(element.label.as_deref(), Some("Save"));
    assert_eq!(element.text.as_deref(), Some("Save"));
    assert_eq!(element.state, ElementState::enabled());
    assert_eq!(element.confidence, 1.0, "the higher confidence wins");

    let provenance = merged.provenance();
    assert!(provenance.rect.source.contains(Source::A11y));
    assert!(provenance.label.source.contains(Source::A11y));
    assert!(provenance.text.source.contains(Source::A11y));
    assert!(provenance.state.source.contains(Source::A11y));
    assert!(!provenance.rect.source.contains(Source::Vision));
}

#[test]
fn vision_supplies_text_when_a11y_is_silent() {
    let engine = MergeEngine::default();
    let a = element(1, 0, 0, 100, 50, Role::StaticText);
    let v = detection(0, 0, 100, 50, Role::StaticText).with_text("Recognized words");

    let report = engine.merge(app(), &[a], &[v]);
    let merged = &report.elements()[0];

    assert_eq!(merged.element().text.as_deref(), Some("Recognized words"));
    assert!(merged.provenance().text.source.contains(Source::Vision));
    assert!(!merged.provenance().text.source.contains(Source::A11y));
    assert_eq!(report.conflicts(), 0);
}

#[test]
fn a11y_wins_the_role_and_the_disagreement_is_flagged() {
    let engine = MergeEngine::default();
    let a = element(1, 0, 0, 100, 100, Role::Button);
    let v = detection(30, 0, 100, 100, Role::Link);

    let report = engine.merge(app(), &[a], &[v]);
    let merged = &report.elements()[0];

    assert_eq!(merged.element().role, Role::Button);
    assert_eq!(report.conflicts(), 1);
    let conflict = &merged.conflicts()[0];
    assert_eq!(conflict.property(), MergeProperty::Role);
    assert_eq!(conflict.a11y(), &ConflictValue::role(&Role::Button));
    assert_eq!(conflict.vision(), &ConflictValue::role(&Role::Link));
    assert!(merged.provenance().role.source.contains(Source::Vision));
    assert_eq!(conflict.describe(), "role: a11y=button, vision=link");
}

#[test]
fn disagreeing_rects_are_unioned_and_flagged() {
    let engine = MergeEngine::default();
    let a = element(1, 0, 0, 100, 100, Role::Button);
    // IoU is 5000 / 15000 = 0.333: enough to match, not enough to agree.
    let v = detection(50, 0, 100, 100, Role::Button);

    let report = engine.merge(app(), &[a], &[v]);
    let merged = &report.elements()[0];

    assert_eq!(merged.element().rect, Rect::new(0, 0, 150, 100));
    assert_eq!(report.conflicts(), 1);
    let conflict = &merged.conflicts()[0];
    assert_eq!(conflict.property(), MergeProperty::Rect);
    assert_eq!(
        conflict.a11y(),
        &ConflictValue::rect(Rect::new(0, 0, 100, 100))
    );
    assert_eq!(
        conflict.vision(),
        &ConflictValue::rect(Rect::new(50, 0, 100, 100))
    );
    assert!(merged.provenance().rect.source.contains(Source::Vision));
    assert!(conflict.describe().contains("rect: a11y="));
}

#[test]
fn disagreeing_text_is_flagged_without_the_content() {
    let engine = MergeEngine::default();
    let mut a = element(1, 0, 0, 100, 50, Role::StaticText);
    a.text = Some("Save".to_string());
    let v = detection(0, 0, 100, 50, Role::StaticText).with_text("Save now");

    let report = engine.merge(app(), &[a], &[v]);
    let merged = &report.elements()[0];

    assert_eq!(merged.element().text.as_deref(), Some("Save"), "a11y wins");
    assert_eq!(report.conflicts(), 1);
    let conflict = &merged.conflicts()[0];
    assert_eq!(conflict.property(), MergeProperty::Text);

    let described = conflict.describe();
    assert!(described.contains("text: a11y="));
    assert!(
        !described.contains("Save"),
        "the content must not be described"
    );
    assert!(!described.contains("now"));
    assert!(described.contains("bytes"));
}

#[test]
fn a_text_conflict_carries_lengths_and_hashes() {
    let value = ConflictValue::content(Some("abcd"));
    match value {
        ConflictValue::Content { len, hash } => {
            assert_eq!(len, 4);
            assert_ne!(hash, 0);
        }
        other => panic!("expected content, got {other:?}"),
    }
    assert_eq!(ConflictValue::content(None), ConflictValue::Absent);

    let mut hasher = DefaultHasher::new();
    "abcd".hash(&mut hasher);
    let expected = format!("4 bytes (hash {})", hasher.finish());
    assert_eq!(ConflictValue::content(Some("abcd")).describe(), expected);
}

#[test]
fn an_incompatible_role_never_matches() {
    let engine = MergeEngine::default();
    let a = element(1, 0, 0, 100, 100, Role::Button);
    let v = detection(0, 0, 100, 100, Role::StaticText);

    let report = engine.merge(app(), &[a], &[v]);
    assert_eq!(report.matched(), 0);
    assert_eq!(report.a11y_only(), 1);
    assert_eq!(report.vision_only(), 1);
}

#[test]
fn the_best_iou_wins_a_contested_element() {
    let engine = MergeEngine::default();
    let a = element(1, 0, 0, 100, 100, Role::Button);
    let exact = detection(0, 0, 100, 100, Role::Button);
    let partial = detection(10, 10, 100, 100, Role::Button);

    let report = engine.merge(app(), std::slice::from_ref(&a), &[exact, partial]);
    assert_eq!(report.matched(), 1);
    assert_eq!(report.vision_only(), 1);
    assert_eq!(
        report.elements()[0].element().rect,
        Rect::new(0, 0, 100, 100)
    );
}

#[test]
fn unmatched_a11y_becomes_semantic_only() {
    let engine = MergeEngine::default();
    let a = element(1, 1_000, 1_000, 10, 10, Role::TextField);

    let report = engine.merge(app(), std::slice::from_ref(&a), &[]);
    let merged = &report.elements()[0];

    assert!(merged.is_semantic_only());
    assert!(!merged.is_matched());
    assert_eq!(merged.element().id, ElementId(1));
    assert_eq!(merged.element().rect, Rect::new(1_000, 1_000, 10, 10));
    assert!(merged.provenance().role.source.contains(Source::A11y));
}

#[test]
fn unmatched_vision_becomes_vision_only() {
    let engine = MergeEngine::default();
    let v = detection(0, 0, 40, 20, Role::Icon).with_text("Home");

    let report = engine.merge(app(), &[], std::slice::from_ref(&v));
    let merged = &report.elements()[0];

    assert!(merged.is_vision_only());
    assert_eq!(merged.element().id, ElementId(DEFAULT_VISION_ID_START));
    assert_eq!(merged.element().rect, Rect::new(0, 0, 40, 20));
    assert_eq!(merged.element().role, Role::Icon);
    assert_eq!(merged.element().text.as_deref(), Some("Home"));
    assert_eq!(merged.element().app, app());
    assert!(merged.provenance().rect.source.contains(Source::Vision));
}

#[test]
fn vision_only_ids_follow_the_input_order() {
    let engine = MergeEngine::default();
    let first = detection(0, 0, 10, 10, Role::Icon);
    let second = detection(20, 0, 10, 10, Role::Icon);

    let report = engine.merge(app(), &[], &[first, second]);
    let ids: Vec<u64> = report
        .elements()
        .iter()
        .map(|merged| merged.element().id.get())
        .collect();
    assert_eq!(
        ids,
        vec![DEFAULT_VISION_ID_START, DEFAULT_VISION_ID_START + 1]
    );
}

#[test]
fn the_merge_is_deterministic() {
    let a = vec![
        element(1, 0, 0, 100, 100, Role::Button),
        element(2, 200, 0, 50, 50, Role::StaticText),
        element(3, 400, 0, 50, 50, Role::Link),
    ];
    let v = vec![
        detection(0, 0, 100, 100, Role::Button),
        detection(210, 5, 50, 50, Role::StaticText),
        detection(600, 0, 50, 50, Role::Icon),
    ];

    let first = MergeEngine::default().merge(app(), &a, &v);
    let second = MergeEngine::default().merge(app(), &a, &v);
    assert_eq!(first, second);
    assert_eq!(first.matched(), 2);
    assert_eq!(first.vision_only(), 1);
}

#[test]
fn empty_inputs_produce_an_empty_graph() {
    let report = MergeEngine::default().merge(app(), &[], &[]);
    assert!(report.is_empty());
    assert_eq!(report.len(), 0);
    assert_eq!(report.matched(), 0);
}

#[test]
fn a_detection_redacts_its_text_in_logs() {
    let detection = detection(0, 0, 10, 10, Role::StaticText).with_text("alpha-bravo");
    let debug = format!("{detection:?}");
    assert!(!debug.contains("alpha-bravo"));
    assert!(debug.contains("text_len"));
}

#[test]
fn merge_properties_are_named() {
    assert_eq!(MergeProperty::Rect.as_str(), "rect");
    assert_eq!(MergeProperty::Text.as_str(), "text");
    assert_eq!(MergeProperty::ALL.len(), 5);
    assert_eq!(MergeProperty::State.to_string(), "state");
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/MERGE.md");
    for word in ["merge", "iou", "provenance", "conflict", "deterministic"] {
        assert!(doc.contains(word), "docs/MERGE.md does not state `{word}`");
    }
}
