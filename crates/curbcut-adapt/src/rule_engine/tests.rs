//! Tests for the forward-chaining rules engine.

use super::*;
use curbcut_core::id::AppId;
use curbcut_core::types::{Element, Media, MediaKind, ScreenGraph};

/// A long, dense sentence that scores as [`ReadingLevel::Complex`].
const COMPLEX_TEXT: &str = "Furthermore, the organizational infrastructure \
     necessitates comprehensive reconsideration of established procedural \
     methodologies and administrative accountability.";

/// A short sentence that scores as [`ReadingLevel::Easy`.
const EASY_TEXT: &str = "The cat sat on the mat. It was a warm day. We sat \
     outside and had some fun.";

fn element(id: u64, x: i32, y: i32, width: u32, height: u32) -> Element {
    Element::new(
        ElementId(id),
        Rect::new(x, y, width, height),
        Role::StaticText,
        AppId::new("fixture"),
    )
}

fn button(id: u64, x: i32, y: i32, width: u32, height: u32, label: &str) -> Element {
    let mut element = element(id, x, y, width, height);
    element.role = Role::Button;
    element.label = Some(label.to_string());
    element
}

fn text_element(id: u64, x: i32, y: i32, width: u32, height: u32, text: &str) -> Element {
    let mut element = element(id, x, y, width, height);
    element.text = Some(text.to_string());
    element
}

fn container(id: u64, x: i32, y: i32, width: u32, height: u32) -> Element {
    let mut element = element(id, x, y, width, height);
    element.role = Role::List;
    element
}

fn graph(elements: Vec<Element>) -> ScreenGraph {
    let mut graph = ScreenGraph::new(AppId::new("fixture"));
    graph.elements = elements;
    graph
}

fn carousel(x: i32, y: i32, width: u32, height: u32) -> Media {
    Media {
        kind: MediaKind::Carousel,
        rect: Rect::new(x, y, width, height),
        motion: true,
    }
}

fn plan(graph: &ScreenGraph, media: &[Media]) -> RulePlan {
    plan_with(graph, media, RuleFlags::default(), RuleToggles::new())
}

fn plan_with(
    graph: &ScreenGraph,
    media: &[Media],
    flags: RuleFlags,
    toggles: RuleToggles,
) -> RulePlan {
    let engine = RuleEngine::new(toggles, flags);
    engine.plan(&RuleContext::new(graph, media, flags))
}

fn kinds(plan: &RulePlan) -> Vec<PlanActionKind> {
    plan.actions().iter().map(|action| action.kind).collect()
}

/// Eight interactive children inside a container.
fn dense_children(offset: u64) -> Vec<Element> {
    (0..8_u64)
        .map(|index| button(offset + index, 10, 10 + index as i32 * 40, 80, 30, "item"))
        .collect()
}

#[test]
fn catalog_is_complete_distinct_and_versioned() {
    assert_eq!(AdaptationRule::ALL.len(), 4);
    let mut ids: Vec<String> = AdaptationRule::ALL
        .iter()
        .map(|rule| rule.id().to_string())
        .collect();
    let count = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), count, "rule ids must be unique");
    for rule in AdaptationRule::ALL {
        assert_eq!(rule.version(), 1);
        assert!(!rule.rationale_text().is_empty());
        assert_eq!(rule.action_kind().conflict_group(), rule.conflict_group());
    }
}

#[test]
fn priorities_are_distinct_in_catalog_order() {
    let mut priorities: Vec<u8> = AdaptationRule::ALL
        .iter()
        .map(|rule| rule.priority())
        .collect();
    let count = priorities.len();
    priorities.sort_unstable();
    priorities.dedup();
    assert_eq!(priorities.len(), count, "v1 priorities must be distinct");
    assert_eq!(priority_of(&RuleId::new("carousel_is_distraction")), 60);
    assert_eq!(priority_of(&RuleId::new("dense_menu_to_grid")), 40);
    assert_eq!(priority_of(&RuleId::new("long_text_chunk")), 20);
    assert_eq!(priority_of(&RuleId::new("destructive_confirm")), 100);
    assert_eq!(priority_of(&RuleId::new("not_a_rule")), 0);
}

#[test]
fn toggles_default_to_every_rule_enabled() {
    let toggles = RuleToggles::new();
    assert_eq!(toggles.disabled_count(), 0);
    assert_eq!(
        toggles.enabled().collect::<Vec<_>>(),
        AdaptationRule::ALL.to_vec()
    );
    for rule in AdaptationRule::ALL {
        assert!(toggles.is_enabled(rule));
    }
}

#[test]
fn a_toggle_disables_and_reenables_one_rule() {
    let mut toggles = RuleToggles::new();
    toggles.disable(AdaptationRule::CarouselIsDistraction);
    assert!(!toggles.is_enabled(AdaptationRule::CarouselIsDistraction));
    assert!(toggles.is_enabled(AdaptationRule::DestructiveConfirm));
    assert_eq!(toggles.disabled_count(), 1);
    toggles.enable(AdaptationRule::CarouselIsDistraction);
    assert!(toggles.is_enabled(AdaptationRule::CarouselIsDistraction));
    assert_eq!(toggles.disabled_count(), 0);
    toggles.set(AdaptationRule::LongTextChunk, false);
    assert_eq!(toggles.enabled().count(), 3);
}

#[test]
fn flags_defaults_are_all_on() {
    let flags = RuleFlags::default();
    assert!(flags.stop_motion);
    assert!(flags.icon_grid);
    assert!(flags.chunking);
    assert!(flags.confirm_actions);
    assert_eq!(flags.reading_target, ReadingLevel::Plain);
    assert_eq!(flags.dense_menu_threshold, DEFAULT_DENSE_MENU_THRESHOLD);
    assert_eq!(flags.min_text_words, DEFAULT_MIN_TEXT_WORDS);
}

#[test]
fn facts_carry_no_text() {
    let mut element = button(1, 0, 0, 200, 40, "Delete my account");
    element.text = Some("Delete my account".to_string());
    let graph = graph(vec![element]);
    let engine = RuleEngine::default();
    let facts = engine.facts(&RuleContext::from_graph(&graph, RuleFlags::default()));
    let rendered = format!("{facts:?}");
    assert!(
        !rendered.contains("Delete"),
        "fact debug output leaked a label: {rendered}"
    );
    assert!(
        !rendered.contains("account"),
        "fact debug output leaked text: {rendered}"
    );
    assert!(facts.contains(&Fact::Destructive { id: ElementId(1) }));
}

#[test]
fn facts_are_deterministic() {
    let graph = graph(vec![button(1, 0, 0, 200, 40, "Delete")]);
    let engine = RuleEngine::default();
    let first = engine.facts(&RuleContext::from_graph(&graph, RuleFlags::default()));
    let second = engine.facts(&RuleContext::from_graph(&graph, RuleFlags::default()));
    assert_eq!(first, second);
}

#[test]
fn a_moving_carousel_is_a_fact_and_a_static_one_is_not() {
    let engine = RuleEngine::default();
    let graph = graph(vec![element(1, 0, 0, 200, 100)]);
    let moving = engine.facts(&RuleContext::new(
        &graph,
        &[carousel(0, 0, 200, 100)],
        RuleFlags::default(),
    ));
    assert!(moving.contains(&Fact::MovingMedia {
        rect: Rect::new(0, 0, 200, 100),
        kind: MediaKind::Carousel,
    }));

    let mut static_carousel = carousel(0, 0, 200, 100);
    static_carousel.motion = false;
    let still = engine.facts(&RuleContext::new(
        &graph,
        &[static_carousel],
        RuleFlags::default(),
    ));
    assert!(!still
        .iter()
        .any(|fact| matches!(fact, Fact::MovingMedia { .. })));

    let video = Media {
        kind: MediaKind::Video,
        rect: Rect::new(0, 0, 200, 100),
        motion: true,
    };
    let video_facts = engine.facts(&RuleContext::new(&graph, &[video], RuleFlags::default()));
    assert!(!video_facts
        .iter()
        .any(|fact| matches!(fact, Fact::MovingMedia { .. })));
}

#[test]
fn a_media_rect_targets_the_element_it_overlaps_most() {
    let graph = graph(vec![element(9, 0, 0, 100, 100), element(5, 0, 0, 50, 50)]);
    let plan = plan(&graph, &[carousel(0, 0, 60, 60)]);
    assert_eq!(kinds(&plan), [PlanActionKind::Freeze]);
    assert_eq!(plan.actions()[0].target, ElementId(9));
}

#[test]
fn equal_overlap_targets_the_smaller_element_then_the_smaller_id() {
    let graph = graph(vec![element(7, 0, 0, 100, 100), element(3, 0, 0, 100, 100)]);
    let plan = plan(&graph, &[carousel(0, 0, 50, 50)]);
    assert_eq!(plan.actions()[0].target, ElementId(3));
}

#[test]
fn media_over_no_element_has_no_target() {
    let graph = graph(vec![element(1, 0, 0, 10, 10)]);
    let plan = plan(&graph, &[carousel(500, 500, 50, 50)]);
    assert!(plan.is_empty());
}

#[test]
fn a_dense_container_counts_interactive_centres() {
    let mut elements = vec![container(1, 0, 0, 200, 400)];
    elements.extend(dense_children(10));
    let graph = graph(elements);
    let engine = RuleEngine::default();
    let facts = engine.facts(&RuleContext::from_graph(&graph, RuleFlags::default()));
    assert_eq!(
        facts,
        vec![Fact::DenseContainer {
            id: ElementId(1),
            interactive: 8,
        }]
    );
}

#[test]
fn a_container_below_the_threshold_is_not_dense() {
    let mut elements = vec![container(1, 0, 0, 200, 400)];
    elements.extend(
        (0..7_u64).map(|index| button(10 + index, 10, 10 + index as i32 * 40, 80, 30, "item")),
    );
    let graph = graph(elements);
    let plan = plan(&graph, &[]);
    assert!(!kinds(&plan).contains(&PlanActionKind::IconGrid));
}

#[test]
fn the_dense_threshold_is_configurable() {
    let mut elements = vec![container(1, 0, 0, 200, 400)];
    elements.extend(dense_children(10));
    let graph = graph(elements);
    let default = plan(&graph, &[]);
    assert_eq!(kinds(&default), [PlanActionKind::IconGrid]);
    let raised = RuleFlags {
        dense_menu_threshold: 9,
        ..RuleFlags::default()
    };
    let plan = plan_with(&graph, &[], raised, RuleToggles::new());
    assert!(!kinds(&plan).contains(&PlanActionKind::IconGrid));
}

#[test]
fn a_nested_container_keeps_the_inner_one() {
    let mut elements = vec![container(1, 0, 0, 200, 400), container(2, 0, 0, 100, 400)];
    elements.extend(dense_children(10));
    let graph = graph(elements);
    let engine = RuleEngine::default();
    let facts = engine.facts(&RuleContext::from_graph(&graph, RuleFlags::default()));
    assert_eq!(
        facts,
        vec![Fact::DenseContainer {
            id: ElementId(2),
            interactive: 8,
        }]
    );
}

#[test]
fn an_unknown_role_is_not_a_container() {
    let mut menu = container(1, 0, 0, 200, 400);
    menu.role = Role::Unknown("menu".to_string());
    let mut elements = vec![menu];
    elements.extend(dense_children(10));
    let graph = graph(elements);
    let plan = plan(&graph, &[]);
    assert!(!kinds(&plan).contains(&PlanActionKind::IconGrid));
}

#[test]
fn text_level_facts_carry_words_and_band_only() {
    let graph = graph(vec![
        text_element(1, 0, 0, 400, 100, COMPLEX_TEXT),
        text_element(2, 0, 200, 400, 100, EASY_TEXT),
    ]);
    let engine = RuleEngine::default();
    let facts = engine.facts(&RuleContext::from_graph(&graph, RuleFlags::default()));
    assert!(facts.contains(&Fact::TextLevel {
        id: ElementId(1),
        words: 14,
        level: ReadingLevel::Complex,
    }));
    assert!(facts.contains(&Fact::TextLevel {
        id: ElementId(2),
        words: 18,
        level: ReadingLevel::Easy,
    }));
}

#[test]
fn an_element_with_no_words_is_not_a_text_fact() {
    let graph = graph(vec![text_element(1, 0, 0, 400, 100, "   ")]);
    let engine = RuleEngine::default();
    let facts = engine.facts(&RuleContext::from_graph(&graph, RuleFlags::default()));
    assert!(!facts
        .iter()
        .any(|fact| matches!(fact, Fact::TextLevel { .. })));
}

#[test]
fn destructive_matching_uses_a_word_boundary() {
    let cases = [
        ("Delete", true),
        ("delete account", true),
        ("Trash", true),
        ("Trash can", true),
        ("Sign out", true),
        ("Factory reset", true),
        ("Deleted", false),
        ("Preset", false),
        ("Catalog out", false),
        ("Undo", false),
    ];
    for (label, expected) in cases {
        let mut elements = vec![button(1, 0, 0, 200, 40, label)];
        let mut text_only = text_element(2, 0, 100, 200, 40, label);
        text_only.role = Role::StaticText;
        elements.push(text_only);
        let graph = graph(elements);
        let plan = plan(&graph, &[]);
        let confirms = kinds(&plan)
            .iter()
            .filter(|kind| **kind == PlanActionKind::Confirm)
            .count();
        assert_eq!(
            confirms,
            usize::from(expected),
            "label {label:?} should yield {expected} confirm(s), got {confirms}"
        );
        assert_eq!(
            plan.actions().first().map(|action| action.target),
            expected.then_some(ElementId(1))
        );
    }
}

#[test]
fn a_carousel_becomes_a_freeze_with_a_rationale() {
    let graph = graph(vec![element(1, 0, 0, 200, 100)]);
    let plan = plan(&graph, &[carousel(0, 0, 200, 100)]);
    assert_eq!(kinds(&plan), [PlanActionKind::Freeze]);
    let action = &plan.actions()[0];
    assert_eq!(action.target, ElementId(1));
    assert_eq!(action.rule().to_string(), "carousel_is_distraction");
    assert!(!action.rationale.text.is_empty());
    assert_eq!(
        plan.rationale_for(ElementId(1)).map(|r| r.rule.to_string()),
        Some("carousel_is_distraction".to_string())
    );
}

#[test]
fn a_dense_menu_becomes_an_icon_grid() {
    let mut elements = vec![container(1, 0, 0, 200, 400)];
    elements.extend(dense_children(10));
    let graph = graph(elements);
    let plan = plan(&graph, &[]);
    assert_eq!(kinds(&plan), [PlanActionKind::IconGrid]);
    assert_eq!(plan.actions()[0].target, ElementId(1));
    assert_eq!(plan.actions()[0].rule().to_string(), "dense_menu_to_grid");
}

#[test]
fn long_complex_text_becomes_a_chunk() {
    let graph = graph(vec![text_element(1, 0, 0, 400, 100, COMPLEX_TEXT)]);
    let plan = plan(&graph, &[]);
    assert_eq!(kinds(&plan), [PlanActionKind::Chunk]);
    assert_eq!(plan.actions()[0].target, ElementId(1));
    assert_eq!(plan.actions()[0].rule().to_string(), "long_text_chunk");
}

#[test]
fn easy_text_and_short_text_are_not_chunked() {
    let graph = graph(vec![
        text_element(1, 0, 0, 400, 100, EASY_TEXT),
        text_element(2, 0, 200, 400, 100, "Short note."),
    ]);
    let plan = plan(&graph, &[]);
    assert!(!kinds(&plan).contains(&PlanActionKind::Chunk));
}

#[test]
fn the_chunk_word_floor_is_configurable() {
    let graph = graph(vec![text_element(1, 0, 0, 400, 100, COMPLEX_TEXT)]);
    let flags = RuleFlags {
        min_text_words: 20,
        ..RuleFlags::default()
    };
    let plan = plan_with(&graph, &[], flags, RuleToggles::new());
    assert!(!kinds(&plan).contains(&PlanActionKind::Chunk));
}

#[test]
fn a_target_at_the_reading_target_is_not_chunked() {
    let graph = graph(vec![text_element(1, 0, 0, 400, 100, COMPLEX_TEXT)]);
    let flags = RuleFlags {
        reading_target: ReadingLevel::Complex,
        ..RuleFlags::default()
    };
    let plan = plan_with(&graph, &[], flags, RuleToggles::new());
    assert!(!kinds(&plan).contains(&PlanActionKind::Chunk));
}

#[test]
fn a_destructive_button_becomes_a_confirm() {
    let graph = graph(vec![button(1, 0, 0, 200, 40, "Delete")]);
    let plan = plan(&graph, &[]);
    assert_eq!(kinds(&plan), [PlanActionKind::Confirm]);
    assert_eq!(plan.actions()[0].rule().to_string(), "destructive_confirm");
}

#[test]
fn a_disabled_rule_derives_no_action_and_never_wins() {
    let carousel_screen = graph(vec![element(1, 0, 0, 200, 100)]);
    let mut toggles = RuleToggles::new();
    toggles.disable(AdaptationRule::CarouselIsDistraction);
    let plan = plan_with(
        &carousel_screen,
        &[carousel(0, 0, 200, 100)],
        RuleFlags::default(),
        toggles,
    );
    assert!(plan.is_empty());

    let mut engine = RuleEngine::default();
    engine.disable(AdaptationRule::DestructiveConfirm);
    let destructive = graph(vec![button(1, 0, 0, 200, 40, "Delete")]);
    let plan = engine.plan(&RuleContext::from_graph(&destructive, RuleFlags::default()));
    assert!(plan.is_empty());
    engine.enable(AdaptationRule::DestructiveConfirm);
    let plan = engine.plan(&RuleContext::from_graph(&destructive, RuleFlags::default()));
    assert_eq!(kinds(&plan), [PlanActionKind::Confirm]);
}

#[test]
fn a_profile_flag_suppresses_its_rule() {
    let graph = graph(vec![element(1, 0, 0, 200, 100)]);
    let flags = RuleFlags {
        stop_motion: false,
        ..RuleFlags::default()
    };
    let plan = plan_with(
        &graph,
        &[carousel(0, 0, 200, 100)],
        flags,
        RuleToggles::new(),
    );
    assert!(plan.is_empty());
}

#[test]
fn confirm_and_freeze_on_one_target_coexist() {
    let graph = graph(vec![button(1, 0, 0, 200, 100, "Delete")]);
    let plan = plan(&graph, &[carousel(0, 0, 200, 100)]);
    assert_eq!(
        kinds(&plan),
        [PlanActionKind::Confirm, PlanActionKind::Freeze]
    );
    assert!(plan.superseded().is_empty());
    assert_eq!(plan.actions_for(ElementId(1)).len(), 2);
}

#[test]
fn a_higher_priority_presentation_wins_and_records_the_loser() {
    let mut dense_text = container(1, 0, 0, 400, 400);
    dense_text.text = Some(COMPLEX_TEXT.to_string());
    let mut elements = vec![dense_text];
    elements.extend(dense_children(10));
    let graph = graph(elements);
    let plan = plan(&graph, &[]);
    assert_eq!(kinds(&plan), [PlanActionKind::IconGrid]);
    assert_eq!(plan.superseded().len(), 1);
    assert_eq!(plan.superseded()[0].reason, SupersedeReason::HigherPriority);
    assert_eq!(
        plan.superseded()[0].winner.to_string(),
        "dense_menu_to_grid"
    );
    assert_eq!(
        plan.superseded()[0].action.rule().to_string(),
        "long_text_chunk"
    );
}

#[test]
fn one_rule_firing_twice_on_one_target_keeps_the_first_claim() {
    let graph = graph(vec![element(1, 0, 0, 200, 200)]);
    let plan = plan(
        &graph,
        &[carousel(0, 0, 200, 100), carousel(0, 100, 200, 100)],
    );
    assert_eq!(kinds(&plan), [PlanActionKind::Freeze]);
    assert_eq!(plan.superseded().len(), 1);
    assert_eq!(plan.superseded()[0].reason, SupersedeReason::SamePriority);
    assert_eq!(
        plan.superseded()[0].winner.to_string(),
        "carousel_is_distraction"
    );
    assert_eq!(plan.superseded()[0].action.target, ElementId(1));
}

#[test]
fn actions_are_sorted_by_priority_then_rule_then_target() {
    let mut dense = container(1, 0, 0, 200, 400);
    dense.text = Some(COMPLEX_TEXT.to_string());
    let mut elements = vec![
        dense,
        button(2, 500, 0, 200, 40, "Delete"),
        element(3, 500, 100, 200, 100),
    ];
    elements.extend(dense_children(10));
    let graph = graph(elements);
    let plan = plan(&graph, &[carousel(500, 100, 200, 100)]);
    assert_eq!(
        kinds(&plan),
        [
            PlanActionKind::Confirm,
            PlanActionKind::Freeze,
            PlanActionKind::IconGrid,
        ]
    );
}

#[test]
fn superseded_actions_are_sorted_by_priority_then_rule_then_target() {
    // Element 1: a dense container under a carousel, so IconGrid loses to Freeze.
    let carousel_dense = container(1, 0, 0, 200, 400);
    // Element 2: a dense container with long text, so Chunk loses to IconGrid.
    let mut text_dense = container(2, 500, 0, 200, 400);
    text_dense.text = Some(COMPLEX_TEXT.to_string());
    let mut elements = vec![carousel_dense, text_dense];
    let first: Vec<Element> = (0..8_u64)
        .map(|index| button(10 + index, 10, 10 + index as i32 * 40, 80, 30, "item"))
        .collect();
    let second: Vec<Element> = (0..8_u64)
        .map(|index| button(30 + index, 510, 10 + index as i32 * 40, 80, 30, "item"))
        .collect();
    elements.extend(first);
    elements.extend(second);
    let graph = graph(elements);
    let plan = plan(&graph, &[carousel(0, 0, 200, 200)]);
    assert_eq!(plan.superseded().len(), 2);
    let losers: Vec<(u8, String)> = plan
        .superseded()
        .iter()
        .map(|entry| (entry.action.priority(), entry.action.rule().to_string()))
        .collect();
    assert_eq!(
        losers,
        vec![
            (40, "dense_menu_to_grid".to_string()),
            (20, "long_text_chunk".to_string()),
        ]
    );
    assert!(plan
        .superseded()
        .iter()
        .all(|entry| entry.reason == SupersedeReason::HigherPriority));
}

#[test]
fn an_empty_graph_plans_nothing() {
    let graph = graph(vec![]);
    let plan = plan(&graph, &[]);
    assert!(plan.is_empty());
    assert_eq!(plan.len(), 0);
    assert_eq!(plan.rationale_for(ElementId(1)), None);
    assert!(plan.actions_for(ElementId(1)).is_empty());
}

#[test]
fn a_media_region_with_no_elements_still_yields_no_action() {
    let graph = graph(vec![]);
    let plan = plan(&graph, &[carousel(0, 0, 100, 100)]);
    assert!(plan.is_empty());
}

#[test]
fn a_button_labelled_with_a_destructive_term_is_the_only_activation() {
    let graph = graph(vec![
        button(1, 0, 0, 200, 40, "Delete"),
        button(2, 0, 100, 200, 40, "Cancel"),
        button(3, 0, 200, 200, 40, "Remove"),
    ]);
    let plan = plan(&graph, &[]);
    assert_eq!(
        kinds(&plan),
        [PlanActionKind::Confirm, PlanActionKind::Confirm]
    );
    let targets: Vec<ElementId> = plan.actions().iter().map(|action| action.target).collect();
    assert_eq!(targets, [ElementId(1), ElementId(3)]);
}

#[test]
fn doc_states_the_rule_catalog_and_the_toggles() {
    let doc = include_str!("../../../../docs/RULES_ENGINE.md");
    for word in [
        "carousel_is_distraction",
        "dense_menu_to_grid",
        "long_text_chunk",
        "destructive_confirm",
        "forward",
        "priority",
        "conflict",
        "supersed",
        "toggle",
        "Rationale",
    ] {
        assert!(
            doc.contains(word),
            "docs/RULES_ENGINE.md does not state `{word}`"
        );
    }
}
