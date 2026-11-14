//! Versioned, user-disableable rules.
//!
//! Every rule has a stable [`RuleId`], a version, and a description. A
//! [`RuleSet`] records which rules the user turned off. A disabled rule
//! produces no action and no [`Rationale`]; an enabled rule always explains
//! itself.

use std::collections::HashSet;
use std::fmt;

use curbcut_core::id::RuleId;
use curbcut_core::types::Rationale;

/// Version of the built-in rule set.
pub const RULE_SET_VERSION: u32 = 1;

/// A deterministic rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rule {
    /// Refine a platform role from visual cues.
    RoleRefinement,
    /// Group reading content into chunks.
    Chunking,
    /// Compare text reading level with a target.
    ReadingLevel,
    /// Score and suppress interruptions.
    Distraction,
}

impl Rule {
    /// Every built-in rule, in stable order.
    pub const ALL: [Rule; 4] = [
        Rule::RoleRefinement,
        Rule::Chunking,
        Rule::ReadingLevel,
        Rule::Distraction,
    ];

    /// The stable id string.
    pub fn name(self) -> &'static str {
        match self {
            Rule::RoleRefinement => "role_refinement",
            Rule::Chunking => "chunking",
            Rule::ReadingLevel => "reading_level",
            Rule::Distraction => "distraction",
        }
    }

    /// The rule id used in [`Rationale`].
    pub fn id(self) -> RuleId {
        RuleId::new(self.name())
    }

    /// The version of this rule's implementation.
    pub fn version(self) -> u32 {
        match self {
            Rule::RoleRefinement | Rule::Chunking | Rule::ReadingLevel | Rule::Distraction => {
                RULE_SET_VERSION
            }
        }
    }

    /// What the rule does, for the provenance panel.
    pub fn description(self) -> &'static str {
        match self {
            Rule::RoleRefinement => "Refine a platform role from visual cues.",
            Rule::Chunking => "Group reading content into chunks of at most N elements.",
            Rule::ReadingLevel => "Compare Flesch reading ease with the profile target.",
            Rule::Distraction => "Score interruptions and suppress the highest band.",
        }
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The enabled and disabled state of the rule set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleSet {
    disabled: HashSet<String>,
}

impl RuleSet {
    /// An empty set: every rule enabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Disables a rule.
    pub fn disable(&mut self, rule: Rule) {
        self.disabled.insert(rule.name().to_string());
    }

    /// Re-enables a rule.
    pub fn enable(&mut self, rule: Rule) {
        self.disabled.remove(rule.name());
    }

    /// Whether a rule is enabled.
    pub fn is_enabled(&self, rule: Rule) -> bool {
        !self.disabled.contains(rule.name())
    }

    /// The enabled rules, in stable order.
    pub fn enabled(&self) -> Vec<Rule> {
        Rule::ALL
            .into_iter()
            .filter(|rule| self.is_enabled(*rule))
            .collect()
    }

    /// The disabled rules, in stable order.
    pub fn disabled(&self) -> Vec<Rule> {
        Rule::ALL
            .into_iter()
            .filter(|rule| !self.is_enabled(*rule))
            .collect()
    }

    /// The rationale for an enabled rule, or `None` when it is disabled.
    ///
    /// `detail` is rule prose about counts and thresholds, never content. The
    /// method is crate-private so an outside caller cannot build a `Rationale`
    /// from screen content; the engine is the only producer.
    pub(crate) fn rationale(&self, rule: Rule, detail: &str) -> Option<Rationale> {
        if !self.is_enabled(rule) {
            return None;
        }
        Some(Rationale::new(
            rule.id(),
            format!("{} v{}: {}", rule.name(), rule.version(), detail),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_has_a_stable_id_version_and_description() {
        for rule in Rule::ALL {
            assert_eq!(rule.id(), RuleId::new(rule.name()));
            assert_eq!(rule.version(), RULE_SET_VERSION);
            assert!(!rule.description().is_empty());
            assert!(!rule.name().is_empty());
        }
    }

    #[test]
    fn a_new_set_enables_every_rule() {
        let rules = RuleSet::new();
        assert_eq!(rules.enabled(), Rule::ALL.to_vec());
        assert!(rules.disabled().is_empty());
        assert!(rules.rationale(Rule::Chunking, "max=6").is_some());
    }

    #[test]
    fn a_disabled_rule_produces_no_rationale() {
        let mut rules = RuleSet::new();
        rules.disable(Rule::Chunking);
        assert!(!rules.is_enabled(Rule::Chunking));
        assert!(rules.is_enabled(Rule::Distraction));
        assert_eq!(rules.disabled(), vec![Rule::Chunking]);
        assert_eq!(rules.enabled().len(), Rule::ALL.len() - 1);
        assert!(rules.rationale(Rule::Chunking, "max=6").is_none());
        assert!(rules.rationale(Rule::Distraction, "score=90").is_some());
    }

    #[test]
    fn a_disabled_rule_can_be_reenabled() {
        let mut rules = RuleSet::new();
        rules.disable(Rule::ReadingLevel);
        rules.enable(Rule::ReadingLevel);
        assert!(rules.is_enabled(Rule::ReadingLevel));
        assert!(rules.disabled().is_empty());
    }

    #[test]
    fn the_rationale_names_the_rule_and_version() {
        let rules = RuleSet::new();
        let rationale = rules
            .rationale(Rule::RoleRefinement, "heading_level=2")
            .expect("enabled");
        assert_eq!(rationale.rule, RuleId::new("role_refinement"));
        assert!(rationale.text.contains("v1"), "{}", rationale.text);
        assert!(
            rationale.text.contains("heading_level=2"),
            "{}",
            rationale.text
        );
    }

    #[test]
    fn rule_names_are_stable() {
        assert_eq!(Rule::RoleRefinement.to_string(), "role_refinement");
        assert_eq!(Rule::Chunking.to_string(), "chunking");
        assert_eq!(Rule::ReadingLevel.to_string(), "reading_level");
        assert_eq!(Rule::Distraction.to_string(), "distraction");
    }
}
