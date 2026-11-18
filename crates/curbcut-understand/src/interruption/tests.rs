//! Tests for interruption detection.

use super::*;
use curbcut_core::id::{AppId, ElementId};
use curbcut_core::types::{Element, Region};

fn screen() -> Rect {
    Rect::new(0, 0, 1000, 800)
}

fn app() -> AppId {
    AppId::new("example")
}

fn element(id: u64, rect: Rect, role: Role) -> Element {
    Element::new(ElementId(id), rect, role, app())
}

fn labelled(element: Element, label: &str) -> Element {
    Element {
        label: Some(label.to_string()),
        ..element
    }
}

fn with_text(element: Element, text: &str) -> Element {
    Element {
        text: Some(text.to_string()),
        ..element
    }
}

fn graph(elements: Vec<Element>, regions: Vec<Region>) -> ScreenGraph {
    ScreenGraph {
        elements,
        regions,
        app: Some(app()),
        trace_id: None,
    }
}

fn kinds(interruptions: &[Interruption]) -> Vec<InterruptionKind> {
    interruptions.iter().map(|item| item.kind).collect()
}

#[test]
fn an_empty_graph_has_no_interruptions() {
    let detected = detect_interruptions(screen(), &graph(Vec::new(), Vec::new()));
    assert!(detected.is_empty());
}

#[test]
fn a_centered_dialog_is_a_popup() {
    let dialog = element(1, Rect::new(400, 300, 200, 200), Role::Dialog);
    let detected = detect_interruptions(screen(), &graph(vec![dialog], Vec::new()));
    assert_eq!(kinds(&detected), vec![InterruptionKind::Popup]);
    assert_eq!(detected[0].rect, Rect::new(400, 300, 200, 200));
    assert!(!detected[0].dismissible);
}

#[test]
fn a_large_surface_is_a_paywall() {
    let dialog = element(1, Rect::new(100, 100, 800, 600), Role::Dialog);
    let detected = detect_interruptions(screen(), &graph(vec![dialog], Vec::new()));
    assert_eq!(kinds(&detected), vec![InterruptionKind::Paywall]);
}

#[test]
fn a_tall_centered_box_is_a_popup_not_a_banner() {
    let dialog = element(1, Rect::new(200, 200, 600, 400), Role::Dialog);
    let detected = detect_interruptions(screen(), &graph(vec![dialog], Vec::new()));
    assert_eq!(kinds(&detected), vec![InterruptionKind::Popup]);
}

#[test]
fn a_bottom_cookie_notice_is_classified() {
    let region = Region {
        kind: RegionKind::Floating,
        rect: Rect::new(0, 700, 1000, 100),
        reading_order: 0,
    };
    let text = with_text(
        element(1, Rect::new(0, 700, 1000, 100), Role::StaticText),
        "We use cookies",
    );
    let detected = detect_interruptions(screen(), &graph(vec![text], vec![region]));
    assert_eq!(kinds(&detected), vec![InterruptionKind::CookieNotice]);
}

#[test]
fn a_bottom_bar_without_a_cookie_word_is_a_banner() {
    let region = Region {
        kind: RegionKind::Floating,
        rect: Rect::new(0, 700, 1000, 100),
        reading_order: 0,
    };
    let detected = detect_interruptions(screen(), &graph(Vec::new(), vec![region]));
    assert_eq!(kinds(&detected), vec![InterruptionKind::Banner]);
}

#[test]
fn a_small_bottom_alert_is_a_toast() {
    let alert = element(1, Rect::new(400, 760, 200, 30), Role::Alert);
    let detected = detect_interruptions(screen(), &graph(vec![alert], Vec::new()));
    assert_eq!(kinds(&detected), vec![InterruptionKind::Toast]);
    assert!(detected[0].dismissible, "an alert is transient");
}

#[test]
fn a_close_button_marks_a_popup_dismissible() {
    let dialog = element(1, Rect::new(400, 300, 200, 200), Role::Dialog);
    let close = labelled(
        element(2, Rect::new(500, 480, 80, 16), Role::Button),
        "Close",
    );
    let detected = detect_interruptions(screen(), &graph(vec![dialog, close], Vec::new()));
    assert_eq!(kinds(&detected), vec![InterruptionKind::Popup]);
    assert!(detected[0].dismissible);
}

#[test]
fn a_button_that_does_not_dismiss_leaves_the_popup_stuck() {
    let dialog = element(1, Rect::new(400, 300, 200, 200), Role::Dialog);
    let go = labelled(element(2, Rect::new(500, 480, 80, 16), Role::Button), "Go");
    let detected = detect_interruptions(screen(), &graph(vec![dialog, go], Vec::new()));
    assert!(!detected[0].dismissible);
}

#[test]
fn a_dialog_inside_a_modal_region_is_reported_once() {
    let region = Region {
        kind: RegionKind::Modal,
        rect: Rect::new(0, 0, 1000, 800),
        reading_order: 0,
    };
    let dialog = element(1, Rect::new(400, 300, 200, 200), Role::Dialog);
    let detected = detect_interruptions(screen(), &graph(vec![dialog], vec![region]));
    assert_eq!(detected.len(), 1, "the nested dialog is part of the modal");
}

#[test]
fn the_result_is_sorted_by_rectangle() {
    let lower = element(1, Rect::new(100, 500, 100, 100), Role::Dialog);
    let upper = element(2, Rect::new(600, 100, 100, 100), Role::Dialog);
    let detected = detect_interruptions(screen(), &graph(vec![lower, upper], Vec::new()));
    let rects: Vec<Rect> = detected.iter().map(|item| item.rect).collect();
    assert_eq!(
        rects,
        vec![Rect::new(600, 100, 100, 100), Rect::new(100, 500, 100, 100)]
    );
}

#[test]
fn detection_is_deterministic() {
    let dialog = element(1, Rect::new(400, 300, 200, 200), Role::Dialog);
    let graph = graph(vec![dialog], Vec::new());
    assert_eq!(
        detect_interruptions(screen(), &graph),
        detect_interruptions(screen(), &graph)
    );
}

#[test]
fn a_lower_paywall_threshold_changes_the_kind() {
    let dialog = element(1, Rect::new(400, 300, 200, 200), Role::Dialog);
    let graph = graph(vec![dialog], Vec::new());
    let config = DetectorConfig {
        paywall_area_ratio: 0.01,
        ..DetectorConfig::default()
    };
    let detector = InterruptionDetector::with_config(screen(), config);
    assert_eq!(detector.detect(&graph)[0].kind, InterruptionKind::Paywall);
    assert_eq!(
        InterruptionDetector::new(screen()).detect(&graph)[0].kind,
        InterruptionKind::Popup
    );
}

#[test]
fn thresholds_are_clamped() {
    let config = DetectorConfig {
        paywall_area_ratio: 2.0,
        banner_width_ratio: -1.0,
        ..DetectorConfig::default()
    };
    let clamped = InterruptionDetector::with_config(screen(), config).config();
    assert_eq!(clamped.paywall_area_ratio, 1.0);
    assert_eq!(clamped.banner_width_ratio, 0.0);
}

#[test]
fn an_empty_screen_still_classifies() {
    let dialog = element(1, Rect::new(400, 300, 200, 200), Role::Dialog);
    let detected = detect_interruptions(Rect::new(0, 0, 0, 0), &graph(vec![dialog], Vec::new()));
    assert_eq!(kinds(&detected), vec![InterruptionKind::Popup]);
}

#[test]
fn content_never_reaches_the_result() {
    let region = Region {
        kind: RegionKind::Floating,
        rect: Rect::new(0, 700, 1000, 100),
        reading_order: 0,
    };
    let text = labelled(
        element(1, Rect::new(0, 700, 1000, 100), Role::StaticText),
        "Cookie settings and tracking",
    );
    let detected = detect_interruptions(screen(), &graph(vec![text], vec![region]));
    let rendered = format!("{:?}", detected[0]);
    assert!(!rendered.contains("cookie"), "the label leaked: {rendered}");
    assert!(
        !rendered.contains("tracking"),
        "the label leaked: {rendered}"
    );
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/INTERRUPTIONS.md");
    for word in [
        "interruption",
        "toast",
        "banner",
        "popup",
        "cookie",
        "paywall",
        "dismissible",
    ] {
        assert!(
            doc.contains(word),
            "docs/INTERRUPTIONS.md does not state `{word}`"
        );
    }
}
