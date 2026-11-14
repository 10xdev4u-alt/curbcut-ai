//! Integration tests for the core type system, exercised through the public API.

use curbcut_core::capability::{require, Capability, Session};
use curbcut_core::error::CurbCutError;
use curbcut_core::id::{AppId, ElementId, SessionId, TraceIdGenerator};
use curbcut_core::types::{Element, Rect, Role, ScreenGraph, Source, SourceSet};

#[test]
fn rect_iou_is_symmetric_and_bounded() {
    let a = Rect::new(0, 0, 10, 10);
    let b = Rect::new(5, 5, 10, 10);
    let iou = a.iou(b);
    assert!((iou - 25.0 / 175.0).abs() < 1e-6);
    assert!((b.iou(a) - iou).abs() < 1e-6);
    assert!((0.0..=1.0).contains(&iou));
    assert_eq!(a.iou(Rect::new(100, 100, 1, 1)), 0.0);
}

#[test]
fn element_debug_redacts_label_and_text() {
    let mut element = Element::new(
        ElementId(1),
        Rect::new(0, 0, 10, 10),
        Role::TextField,
        AppId::new("mail"),
    );
    element.label = Some("SECRET_LABEL".to_string());
    element.text = Some("SECRET_TEXT".to_string());
    let rendered = format!("{element:?}");
    assert!(!rendered.contains("SECRET_LABEL"));
    assert!(!rendered.contains("SECRET_TEXT"));
    assert!(rendered.contains("label_len"));
    assert!(rendered.contains("text_len"));
}

#[test]
fn screengraph_redacted_view_has_no_content() {
    let mut graph = ScreenGraph::new(AppId::new("mail"));
    let mut element = Element::new(
        ElementId(1),
        Rect::new(0, 0, 10, 10),
        Role::StaticText,
        AppId::new("mail"),
    );
    element.text = Some("SECRET_TEXT".to_string());
    graph.elements.push(element);
    let rendered = format!("{:?}", graph.redacted());
    assert!(!rendered.contains("SECRET_TEXT"));
}

#[test]
fn source_set_merges_sources() {
    let mut set = SourceSet::from(Source::A11y);
    set.insert(Source::Vision);
    assert_eq!(set.len(), 2);
    assert!(set.contains(Source::A11y));
    assert!(set.contains(Source::Vision));
    assert!(!set.contains(Source::Merged));
}

#[test]
fn capability_tokens_are_not_cloneable_and_expire_on_revoke() {
    let mut session = Session::new(SessionId(7));
    let token = session.grant(Capability::Act);
    require(&token, Capability::Act).expect("granted token must satisfy require");
    session.revoke(Capability::Act);
    assert!(require(&token, Capability::Act).is_err());
    assert!(require(&token, Capability::Observe).is_err());
}

#[test]
fn errors_do_not_leak_values() {
    let err = CurbCutError::InvalidInput {
        field: "active_profile".to_string(),
    };
    let rendered = err.to_string();
    assert!(rendered.contains("active_profile"));
    assert!(!rendered.contains("SECRET"));
    assert_eq!(err.summary(), "invalid input");
}

#[test]
fn trace_ids_are_unique() {
    let generator = TraceIdGenerator::new(0);
    let ids: std::collections::BTreeSet<_> = (0..1000).map(|_| generator.next_id()).collect();
    assert_eq!(ids.len(), 1000);
}
