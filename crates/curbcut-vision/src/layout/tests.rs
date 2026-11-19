//! Tests for layout analysis.

use super::*;
use curbcut_core::id::AppId;
use curbcut_core::types::{Element, Region};

fn viewport() -> Rect {
    Rect::new(0, 0, 1000, 800)
}

fn element(id: u64, x: i32, y: i32, width: u32, height: u32) -> Element {
    Element::new(
        ElementId(id),
        Rect::new(x, y, width, height),
        Role::StaticText,
        AppId::new("fixture"),
    )
}

fn graph(elements: Vec<Element>) -> ScreenGraph {
    let mut graph = ScreenGraph::new(AppId::new("fixture"));
    graph.elements = elements;
    graph
}

fn analyze(graph: &ScreenGraph) -> LayoutAnalysis {
    LayoutAnalyzer::new(viewport(), LayoutConfig::default()).analyze(graph)
}

fn kinds(analysis: &LayoutAnalysis) -> Vec<RegionKind> {
    analysis
        .regions()
        .iter()
        .map(|region| region.kind)
        .collect()
}

fn modal(id: u64, x: i32, y: i32, width: u32, height: u32) -> Element {
    let mut element = element(id, x, y, width, height);
    element.role = Role::Dialog;
    element
}

#[test]
fn a_full_screen_element_is_main() {
    let analysis = analyze(&graph(vec![element(1, 0, 0, 1000, 800)]));
    assert_eq!(kinds(&analysis), [RegionKind::Main]);
    assert_eq!(analysis.reading_order(), [ElementId(1)]);
    assert_eq!(analysis.region_of(ElementId(1)), Some(RegionKind::Main));
}

#[test]
fn header_nav_main_and_footer_are_classified() {
    let analysis = analyze(&graph(vec![
        element(1, 0, 0, 1000, 100),
        element(2, 0, 100, 200, 600),
        element(3, 200, 100, 700, 600),
        element(4, 0, 700, 1000, 100),
    ]));
    assert_eq!(
        kinds(&analysis),
        [
            RegionKind::Header,
            RegionKind::Nav,
            RegionKind::Main,
            RegionKind::Footer
        ]
    );
    assert_eq!(
        analysis.reading_order(),
        [ElementId(1), ElementId(2), ElementId(3), ElementId(4)]
    );
    for (index, region) in analysis.regions().iter().enumerate() {
        assert_eq!(region.element_ids.len(), 1);
        assert_eq!(region.reading_order, index);
    }
}

#[test]
fn an_aside_is_classified() {
    let analysis = analyze(&graph(vec![
        element(1, 0, 0, 800, 800),
        element(2, 800, 0, 200, 800),
    ]));
    assert_eq!(kinds(&analysis), [RegionKind::Main, RegionKind::Aside]);
    assert_eq!(analysis.region_of(ElementId(2)), Some(RegionKind::Aside));
}

#[test]
fn a_centered_dialog_is_a_modal() {
    let analysis = analyze(&graph(vec![
        element(1, 0, 0, 1000, 800),
        modal(2, 300, 300, 400, 200),
    ]));
    assert_eq!(kinds(&analysis), [RegionKind::Main, RegionKind::Modal]);
    assert_eq!(analysis.region_of(ElementId(1)), Some(RegionKind::Main));
    assert_eq!(analysis.region_of(ElementId(2)), Some(RegionKind::Modal));
}

#[test]
fn an_off_centre_overlay_is_floating() {
    let analysis = analyze(&graph(vec![
        element(1, 0, 0, 1000, 800),
        element(2, 50, 50, 100, 100),
    ]));
    assert_eq!(kinds(&analysis), [RegionKind::Main, RegionKind::Floating]);
    assert_eq!(analysis.region_of(ElementId(2)), Some(RegionKind::Floating));
}

#[test]
fn the_largest_region_is_never_a_modal() {
    let mut graph = graph(vec![element(1, 0, 0, 1000, 800)]);
    graph.regions.push(Region {
        kind: RegionKind::Modal,
        rect: viewport(),
        reading_order: 0,
    });
    let analysis = analyze(&graph);
    assert_eq!(kinds(&analysis), [RegionKind::Main]);
    assert_eq!(analysis.region_count(), 1);
}

#[test]
fn nested_region_hints_are_deduplicated() {
    let mut graph = graph(vec![
        element(1, 0, 100, 500, 400),
        element(2, 300, 300, 50, 50),
    ]);
    graph.regions.push(Region {
        kind: RegionKind::Modal,
        rect: Rect::new(100, 100, 500, 400),
        reading_order: 0,
    });
    graph.regions.push(Region {
        kind: RegionKind::Modal,
        rect: Rect::new(150, 150, 100, 100),
        reading_order: 1,
    });
    let analysis = analyze(&graph);
    let modals: Vec<_> = analysis
        .regions()
        .iter()
        .filter(|region| region.kind == RegionKind::Modal)
        .collect();
    assert_eq!(modals.len(), 1);
    assert_eq!(modals[0].rect, Rect::new(100, 100, 500, 400));
}

#[test]
fn every_element_is_assigned_once() {
    let analysis = analyze(&graph(vec![
        element(1, 0, 0, 1000, 100),
        element(2, 0, 100, 200, 600),
        element(3, 200, 100, 700, 600),
        element(4, 0, 700, 1000, 100),
    ]));
    let mut assigned: Vec<ElementId> = analysis
        .regions()
        .iter()
        .flat_map(|region| region.element_ids.iter().copied())
        .collect();
    assigned.sort();
    assert_eq!(
        assigned,
        [ElementId(1), ElementId(2), ElementId(3), ElementId(4)]
    );
    assert_eq!(analysis.reading_order().len(), 4);
}

#[test]
fn reading_order_follows_position() {
    let analysis = analyze(&graph(vec![
        element(1, 200, 200, 100, 50),
        element(2, 200, 100, 100, 50),
    ]));
    assert_eq!(analysis.reading_order(), [ElementId(2), ElementId(1)]);
}

#[test]
fn an_empty_viewport_has_no_regions() {
    let analyzer = LayoutAnalyzer::new(Rect::new(0, 0, 0, 800), LayoutConfig::default());
    let analysis = analyzer.analyze(&graph(vec![element(1, 0, 0, 100, 100)]));
    assert!(analysis.is_empty());
    assert_eq!(analysis.region_count(), 0);
    assert!(analysis.reading_order().is_empty());
}

#[test]
fn elements_outside_the_viewport_are_ignored() {
    let analysis = analyze(&graph(vec![
        element(1, -500, -500, 100, 100),
        element(2, 0, 0, 1000, 800),
    ]));
    assert_eq!(analysis.reading_order(), [ElementId(2)]);
    assert_eq!(analysis.region_of(ElementId(1)), None);
}

#[test]
fn the_config_is_clamped() {
    let config = LayoutConfig {
        header_height_ratio: f32::NAN,
        sidebar_width_ratio: 2.0,
        containment_ratio: -1.0,
        ..LayoutConfig::default()
    }
    .clamped();
    assert_eq!(config.header_height_ratio, DEFAULT_HEADER_HEIGHT_RATIO);
    assert_eq!(config.sidebar_width_ratio, 1.0);
    assert_eq!(config.containment_ratio, 0.0);
    let analyzer = LayoutAnalyzer::new(viewport(), config);
    assert_eq!(analyzer.config().sidebar_width_ratio, 1.0);
}

#[test]
fn a_region_below_the_minimum_area_is_ignored() {
    let config = LayoutConfig {
        region_min_area_ratio: 0.5,
        ..LayoutConfig::default()
    };
    let analyzer = LayoutAnalyzer::new(viewport(), config);
    let analysis = analyzer.analyze(&graph(vec![
        element(1, 0, 0, 1000, 100),
        element(2, 0, 100, 200, 600),
        element(3, 200, 100, 700, 600),
        element(4, 0, 700, 1000, 100),
    ]));
    assert_eq!(kinds(&analysis), [RegionKind::Main]);
}

#[test]
fn overlapping_elements_go_to_the_most_specific_region() {
    let analysis = analyze(&graph(vec![
        element(1, 0, 0, 1000, 800),
        modal(2, 300, 300, 400, 200),
    ]));
    assert_eq!(analysis.region_of(ElementId(1)), Some(RegionKind::Main));
    assert_eq!(analysis.region_of(ElementId(2)), Some(RegionKind::Modal));
    assert_eq!(analysis.region_of(ElementId(999)), None);
}

#[test]
fn a_region_with_no_elements_is_not_emitted() {
    let mut graph = graph(vec![element(1, 0, 0, 1000, 800)]);
    graph.regions.push(Region {
        kind: RegionKind::Aside,
        rect: Rect::new(900, 0, 100, 800),
        reading_order: 0,
    });
    let analysis = analyze(&graph);
    assert_eq!(kinds(&analysis), [RegionKind::Main]);
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/LAYOUT_ANALYSIS.md");
    for word in [
        "header",
        "nav",
        "main",
        "aside",
        "footer",
        "modal",
        "floating",
        "reading order",
    ] {
        assert!(
            doc.contains(word),
            "docs/LAYOUT_ANALYSIS.md does not state `{word}`"
        );
    }
}
