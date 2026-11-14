//! The deterministic rule engine.
//!
//! [`RuleEngine`] applies the enabled rules and attaches a [`Rationale`] to
//! every result. A disabled rule still computes its result, so the caller can
//! show what it would have done, but the rationale is `None` and the action is
//! not taken.

use curbcut_core::types::{Element, Interruption, Rationale, ReadingLevel, Role};

use crate::chunking::{self, Chunk};
use crate::distraction::{self, DistractionScore};
use crate::reading::{self, ReadingVerdict};
use crate::roles::{self, Cues};
use crate::rule::{Rule, RuleSet};

/// Default maximum elements per chunk.
pub const DEFAULT_MAX_CHUNK: usize = 6;
/// Default reading target.
pub const DEFAULT_READING_TARGET: ReadingLevel = ReadingLevel::Plain;
/// Default distraction threshold.
pub const DEFAULT_DISTRACTION_THRESHOLD: u8 = 50;

/// Applies the enabled rules and explains every result.
#[derive(Debug, Clone)]
pub struct RuleEngine {
    rules: RuleSet,
    max_chunk: usize,
    reading_target: ReadingLevel,
    distraction_threshold: u8,
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new(RuleSet::new())
    }
}

impl RuleEngine {
    /// An engine over a rule set, with the default thresholds.
    pub fn new(rules: RuleSet) -> Self {
        Self {
            rules,
            max_chunk: DEFAULT_MAX_CHUNK,
            reading_target: DEFAULT_READING_TARGET,
            distraction_threshold: DEFAULT_DISTRACTION_THRESHOLD,
        }
    }

    /// The rule set.
    pub fn rules(&self) -> &RuleSet {
        &self.rules
    }

    /// The maximum elements per chunk.
    pub fn max_chunk(&self) -> usize {
        self.max_chunk
    }

    /// Sets the maximum elements per chunk.
    pub fn set_max_chunk(&mut self, max_chunk: usize) {
        self.max_chunk = max_chunk;
    }

    /// The reading target.
    pub fn reading_target(&self) -> ReadingLevel {
        self.reading_target
    }

    /// Sets the reading target.
    pub fn set_reading_target(&mut self, target: ReadingLevel) {
        self.reading_target = target;
    }

    /// The distraction threshold.
    pub fn distraction_threshold(&self) -> u8 {
        self.distraction_threshold
    }

    /// Sets the distraction threshold.
    pub fn set_distraction_threshold(&mut self, threshold: u8) {
        self.distraction_threshold = threshold;
    }

    /// Refines a role and explains it.
    pub fn refine(&self, role: Role, cues: &Cues) -> (Role, Option<Rationale>) {
        let refined = roles::refine_role(role, cues);
        let rationale = self
            .rules
            .rationale(Rule::RoleRefinement, &format!("cues={}", cues.summary()));
        (refined, rationale)
    }

    /// Chunks reading content and explains it.
    pub fn chunk(&self, elements: &[Element]) -> (Vec<Chunk>, Option<Rationale>) {
        let chunks = chunking::chunk_elements(elements, self.max_chunk);
        let rationale = self.rules.rationale(
            Rule::Chunking,
            &format!("chunks={}, max={}", chunks.len(), self.max_chunk),
        );
        (chunks, rationale)
    }

    /// Compares reading level and explains it.
    pub fn reading(&self, text: &str) -> (ReadingVerdict, Option<Rationale>) {
        let verdict = reading::compare(text, self.reading_target);
        let rationale = self.rules.rationale(
            Rule::ReadingLevel,
            &format!(
                "score={:.1}, level={:?}, target={:?}",
                verdict.score, verdict.level, verdict.target
            ),
        );
        (verdict, rationale)
    }

    /// Scores an interruption and explains it.
    pub fn distraction(
        &self,
        interruption: &Interruption,
    ) -> (DistractionScore, Option<Rationale>) {
        let scored = distraction::score_with(interruption, self.distraction_threshold);
        let rationale = self.rules.rationale(
            Rule::Distraction,
            &format!(
                "score={}, band={:?}, threshold={}, suppress={}",
                scored.score, scored.band, self.distraction_threshold, scored.suppress
            ),
        );
        (scored, rationale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distraction::DistractionBand;
    use curbcut_core::id::{AppId, ElementId};
    use curbcut_core::types::{InterruptionKind, Rect};

    fn element(id: u64, role: Role) -> Element {
        Element::new(
            ElementId(id),
            Rect::new(0, 0, 10, 10),
            role,
            AppId::new("test"),
        )
    }

    fn interruption(kind: InterruptionKind) -> Interruption {
        Interruption {
            kind,
            rect: Rect::new(0, 0, 10, 10),
            dismissible: true,
        }
    }

    #[test]
    fn defaults_are_sensible() {
        let engine = RuleEngine::default();
        assert_eq!(engine.max_chunk(), DEFAULT_MAX_CHUNK);
        assert_eq!(engine.reading_target(), DEFAULT_READING_TARGET);
        assert_eq!(
            engine.distraction_threshold(),
            DEFAULT_DISTRACTION_THRESHOLD
        );
        assert_eq!(engine.rules().enabled().len(), Rule::ALL.len());
    }

    #[test]
    fn every_rule_explains_itself_when_enabled() {
        let engine = RuleEngine::default();
        let cues = Cues {
            heading_level: Some(1),
            ..Cues::none()
        };
        let (role, rationale) = engine.refine(Role::StaticText, &cues);
        assert_eq!(role, Role::Heading);
        assert!(rationale.is_some(), "role refinement must explain itself");

        let elements = vec![element(1, Role::StaticText), element(2, Role::StaticText)];
        let (chunks, rationale) = engine.chunk(&elements);
        assert_eq!(chunks.len(), 1);
        assert!(rationale.is_some());

        let (_, rationale) = engine.reading("The cat sat on the mat.");
        assert!(rationale.is_some());

        let (_, rationale) = engine.distraction(&interruption(InterruptionKind::Paywall));
        assert!(rationale.is_some());
    }

    #[test]
    fn a_disabled_rule_returns_no_rationale() {
        let mut rules = RuleSet::new();
        rules.disable(Rule::Chunking);
        let engine = RuleEngine::new(rules);
        let elements = vec![element(1, Role::StaticText)];
        let (chunks, rationale) = engine.chunk(&elements);
        assert_eq!(chunks.len(), 1, "the result is still computed");
        assert!(rationale.is_none(), "a disabled rule explains nothing");
    }

    #[test]
    fn thresholds_are_configurable() {
        let mut engine = RuleEngine::default();
        engine.set_max_chunk(1);
        engine.set_reading_target(ReadingLevel::Easy);
        engine.set_distraction_threshold(90);

        let elements = vec![element(1, Role::StaticText), element(2, Role::StaticText)];
        let (chunks, _) = engine.chunk(&elements);
        assert_eq!(chunks.len(), 2, "max_chunk=1 splits every element");

        let (verdict, _) = engine.reading("The cat sat on the mat.");
        assert_eq!(verdict.target, ReadingLevel::Easy);

        let (scored, rationale) = engine.distraction(&interruption(InterruptionKind::Paywall));
        assert_eq!(scored.band, DistractionBand::High);
        assert!(scored.suppress, "score 90 meets threshold 90");
        let rationale = rationale.expect("enabled");
        assert!(
            rationale.text.contains("suppress=true"),
            "{}",
            rationale.text
        );
    }

    #[test]
    fn the_threshold_changes_the_suppression_decision() {
        let mut engine = RuleEngine::default();
        engine.set_distraction_threshold(40);
        let banner = Interruption {
            kind: InterruptionKind::Banner,
            rect: Rect::new(0, 0, 10, 10),
            dismissible: true,
        };
        assert!(engine.distraction(&banner).0.suppress);

        engine.set_distraction_threshold(41);
        assert!(!engine.distraction(&banner).0.suppress);
    }

    #[test]
    fn disabling_any_rule_removes_only_its_rationale() {
        for disabled in Rule::ALL {
            let mut rules = RuleSet::new();
            rules.disable(disabled);
            let engine = RuleEngine::new(rules);

            let cues = Cues {
                heading_level: Some(1),
                ..Cues::none()
            };
            let elements = vec![element(1, Role::StaticText)];
            let interruption = interruption(InterruptionKind::Banner);
            let rationales = [
                engine.refine(Role::StaticText, &cues).1,
                engine.chunk(&elements).1,
                engine.reading("The cat sat on the mat.").1,
                engine.distraction(&interruption).1,
            ];
            for (rule, rationale) in Rule::ALL.into_iter().zip(rationales) {
                if rule == disabled {
                    assert!(rationale.is_none(), "{disabled} is disabled");
                } else {
                    assert!(rationale.is_some(), "{rule} is still enabled");
                }
            }
        }
    }

    #[test]
    fn the_rationale_names_the_rule() {
        let engine = RuleEngine::default();
        let (_, rationale) = engine.reading("The cat sat on the mat.");
        let rationale = rationale.expect("enabled");
        assert_eq!(rationale.rule, Rule::ReadingLevel.id());
        assert!(rationale.text.contains("score="), "{}", rationale.text);
    }
}
