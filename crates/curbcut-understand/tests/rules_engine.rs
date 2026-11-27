//! Integration tests for the deterministic rule engine through the public API.
//!
//! These call the engine the way the adapt pipeline does: build elements,
//! apply the enabled rules, and assert on the returned results and rationales.

use curbcut_core::id::{AppId, ElementId};
use curbcut_core::types::{
    Element, Interruption, InterruptionKind, Rationale, ReadingLevel, Rect, Role,
};
use curbcut_understand::chunking::Chunk;
use curbcut_understand::distraction::DistractionBand;
use curbcut_understand::engine::{RuleEngine, DEFAULT_DISTRACTION_THRESHOLD, DEFAULT_MAX_CHUNK};
use curbcut_understand::reading::ReadingVerdict;
use curbcut_understand::roles::Cues;
use curbcut_understand::rule::{Rule, RuleSet, RULE_SET_VERSION};

fn element(id: u64, role: Role) -> Element {
    Element::new(
        ElementId(id),
        Rect::new(0, 0, 10, 10),
        role,
        AppId::new("test"),
    )
}

fn rule_ids(rationales: &[Rationale]) -> Vec<String> {
    rationales
        .iter()
        .map(|rationale| rationale.rule.to_string())
        .collect()
}

#[test]
fn the_default_engine_explains_every_rule() {
    let engine = RuleEngine::default();
    assert_eq!(engine.max_chunk(), DEFAULT_MAX_CHUNK);
    assert_eq!(
        engine.distraction_threshold(),
        DEFAULT_DISTRACTION_THRESHOLD
    );

    let (role, rationale) = engine.refine(
        Role::StaticText,
        &Cues {
            heading_level: Some(1),
            ..Cues::none()
        },
    );
    assert_eq!(role, Role::Heading);
    assert_eq!(rationale.expect("enabled").rule, Rule::RoleRefinement.id());

    let elements = vec![element(1, Role::StaticText), element(2, Role::StaticText)];
    let (chunks, rationale) = engine.chunk(&elements);
    assert_eq!(chunks.len(), 1);
    assert_eq!(rationale.expect("enabled").rule, Rule::Chunking.id());

    let (verdict, rationale) = engine.reading("The cat sat on the mat.");
    assert_eq!(verdict.level, ReadingLevel::Easy);
    assert_eq!(rationale.expect("enabled").rule, Rule::ReadingLevel.id());

    let (score, rationale) = engine.distraction(&Interruption {
        kind: InterruptionKind::Paywall,
        rect: Rect::new(0, 0, 10, 10),
        dismissible: false,
    });
    assert_eq!(score.band, DistractionBand::High);
    assert_eq!(rationale.expect("enabled").rule, Rule::Distraction.id());
}

#[test]
fn a_disabled_rule_still_computes_but_does_not_explain() {
    let mut rules = RuleSet::new();
    rules.disable(Rule::Chunking);
    let engine = RuleEngine::new(rules);

    let elements = vec![element(1, Role::StaticText), element(2, Role::StaticText)];
    let (chunks, rationale) = engine.chunk(&elements);
    assert_eq!(chunks.len(), 1, "the result is still available");
    assert!(
        rationale.is_none(),
        "a disabled rule must not produce a rationale"
    );
}

#[test]
fn the_rule_set_reports_versions_and_order() {
    assert_eq!(RULE_SET_VERSION, 2);
    assert_eq!(Rule::ALL.len(), 4);
    let rules = RuleSet::new();
    assert_eq!(rules.enabled().len(), Rule::ALL.len());
    assert!(rules.disabled().is_empty());

    let mut rules = RuleSet::new();
    rules.disable(Rule::Distraction);
    assert_eq!(rules.disabled(), vec![Rule::Distraction]);
    assert_eq!(rule_ids(&[]), Vec::<String>::new());
}

#[test]
fn chunks_respect_the_heading_boundary() {
    let elements = vec![
        element(1, Role::Heading),
        element(2, Role::StaticText),
        element(3, Role::Heading),
        element(4, Role::StaticText),
    ];
    let mut engine = RuleEngine::default();
    engine.set_max_chunk(6);
    let (chunks, _) = engine.chunk(&elements);
    let ids: Vec<Vec<u64>> = chunks
        .iter()
        .map(|Chunk { element_ids, .. }| element_ids.iter().map(|id| id.get()).collect())
        .collect();
    assert_eq!(ids, vec![vec![1, 2], vec![3, 4]]);
    assert!(chunks.iter().all(|chunk| chunk.starts_section));
}

#[test]
fn the_reading_rule_reports_a_verdict_and_a_rationale() {
    let engine = RuleEngine::default();
    let (verdict, rationale): (ReadingVerdict, Option<Rationale>) =
        engine.reading("The implementation necessitates a comprehensive reconceptualization.");
    assert_eq!(verdict.level, ReadingLevel::Complex);
    assert!(!verdict.meets_target);
    let rationale = rationale.expect("enabled");
    assert!(rationale.text.contains("score="), "{}", rationale.text);
    assert!(
        rationale.text.contains("target=Plain"),
        "{}",
        rationale.text
    );
}

#[test]
fn the_distraction_rule_uses_the_threshold() {
    let mut engine = RuleEngine::default();
    engine.set_distraction_threshold(90);
    let banner = Interruption {
        kind: InterruptionKind::Banner,
        rect: Rect::new(0, 0, 10, 10),
        dismissible: true,
    };
    let (score, _) = engine.distraction(&banner);
    assert_eq!(score.score, 40);
    assert_eq!(score.band, DistractionBand::Low);
    assert!(!score.suppress, "threshold 90 is above the score 40");

    engine.set_distraction_threshold(40);
    assert!(engine.distraction(&banner).0.suppress);

    assert!(curbcut_understand::distraction::should_suppress(
        &banner, 40
    ));
    assert!(!curbcut_understand::distraction::should_suppress(
        &banner, 41
    ));
}
