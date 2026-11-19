//! Forward-chaining adaptation rules.
//!
//! [`RuleEngine`] derives content-free facts from a [`ScreenGraph`] and the
//! media regions the perception layer reported, evaluates the built-in rules,
//! and returns a [`RulePlan`]. See
//! [`../../../docs/RULES_ENGINE.md`](../../../docs/RULES_ENGINE.md).
//!
//! Five principles hold:
//!
//! - **Facts first.** [`RuleEngine::facts`] derives every fact from the graph
//!   and the media list before any rule runs. A fact carries ids, rectangles,
//!   kinds, word counts, and reading bands — never text.
//! - **Rules are data.** [`AdaptationRule`] is a closed catalog with a stable
//!   id, a version, and a priority. A rule is a pure function of the facts and
//!   the profile flags; it has no clock, no I/O, and no hidden state.
//! - **Priority resolves conflicts.** Rules run in catalog order and offer their
//!   actions to a working set of claims. A later offer for a target and
//!   [`ConflictGroup`] a higher-priority rule already holds replaces the claim;
//!   every replaced or refused offer is recorded in [`RulePlan::superseded`]
//!   with the winner. The working set is the forward-chaining memory: a
//!   conclusion an earlier rule reached constrains every rule after it.
//! - **Every action explains itself.** [`PlanAction::rationale`] carries the
//!   rule id and fixed rule prose. No action text is derived from the screen.
//! - **A rule can be turned off.** [`RuleToggles`] is the user's per-rule
//!   toggle. A disabled rule derives no action and is never a winner.
//!
//! The module never logs. It reads `Element::label` and `Element::text` in
//! memory to derive facts and never stores, formats, or prints them.

use std::cmp::Reverse;

use curbcut_core::id::{ElementId, RuleId};
use curbcut_core::types::{
    Element, Media, MediaKind, Point, Rationale, ReadingLevel, Rect, Role, ScreenGraph,
};
use curbcut_understand::reading::reading_level;

/// Version of the built-in adaptation rule set.
pub const ADAPTATION_RULE_SET_VERSION: u32 = 1;

/// Default number of interactive elements that makes a container dense.
pub const DEFAULT_DENSE_MENU_THRESHOLD: usize = 8;

/// Default number of words a text must have before it can be chunked.
pub const DEFAULT_MIN_TEXT_WORDS: usize = 12;

/// The fixed destructive-control terms, matched on a word boundary.
///
/// This is rule data, not screen content: it is a fixed list compiled into the
/// binary, never read from the screen.
pub const DESTRUCTIVE_TERMS: &[&str] = &[
    "delete",
    "remove",
    "erase",
    "discard",
    "uninstall",
    "format",
    "reset",
    "sign out",
    "log out",
    "deactivate",
    "close account",
    "trash",
    "empty trash",
    "clear all",
    "factory reset",
];

/// A closed catalog of the built-in adaptation rules, in catalog order.
///
/// Catalog order is the order the rules are declared in
/// [`docs/RULES_ENGINE.md`](../../../docs/RULES_ENGINE.md). It is deliberately
/// not priority order: priority decides a conflict, not the order of
/// evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AdaptationRule {
    /// Cover a moving carousel with a static snapshot.
    CarouselIsDistraction,
    /// Present a dense menu as an icon grid.
    DenseMenuToGrid,
    /// Break reading content above the target level into chunks.
    LongTextChunk,
    /// Ask for confirmation before a destructive control acts.
    DestructiveConfirm,
}

impl AdaptationRule {
    /// Every built-in rule, in catalog order.
    pub const ALL: [AdaptationRule; 4] = [
        AdaptationRule::CarouselIsDistraction,
        AdaptationRule::DenseMenuToGrid,
        AdaptationRule::LongTextChunk,
        AdaptationRule::DestructiveConfirm,
    ];

    /// The stable id string.
    pub fn id(self) -> RuleId {
        RuleId::new(self.name())
    }

    /// The stable name.
    pub const fn name(self) -> &'static str {
        match self {
            AdaptationRule::CarouselIsDistraction => "carousel_is_distraction",
            AdaptationRule::DenseMenuToGrid => "dense_menu_to_grid",
            AdaptationRule::LongTextChunk => "long_text_chunk",
            AdaptationRule::DestructiveConfirm => "destructive_confirm",
        }
    }

    /// The rule's priority. Higher wins a conflict.
    pub const fn priority(self) -> u8 {
        match self {
            AdaptationRule::CarouselIsDistraction => 60,
            AdaptationRule::DenseMenuToGrid => 40,
            AdaptationRule::LongTextChunk => 20,
            AdaptationRule::DestructiveConfirm => 100,
        }
    }

    /// The version of the rule's behaviour.
    ///
    /// A change to what a rule does is a version bump, not a silent edit, so a
    /// stored [`Rationale`] stays interpretable.
    pub const fn version(self) -> u32 {
        1
    }

    /// The kind of action the rule produces.
    pub const fn action_kind(self) -> PlanActionKind {
        match self {
            AdaptationRule::CarouselIsDistraction => PlanActionKind::Freeze,
            AdaptationRule::DenseMenuToGrid => PlanActionKind::IconGrid,
            AdaptationRule::LongTextChunk => PlanActionKind::Chunk,
            AdaptationRule::DestructiveConfirm => PlanActionKind::Confirm,
        }
    }

    /// The conflict group the rule's action occupies.
    pub const fn conflict_group(self) -> ConflictGroup {
        match self {
            AdaptationRule::DestructiveConfirm => ConflictGroup::Activation,
            _ => ConflictGroup::Presentation,
        }
    }

    /// The fixed rationale prose. Rule prose, never screen content.
    pub const fn rationale_text(self) -> &'static str {
        match self {
            AdaptationRule::CarouselIsDistraction => {
                "A moving carousel is covered with a static snapshot until you bring it back."
            }
            AdaptationRule::DenseMenuToGrid => {
                "A dense menu is presented as a small number of large labelled tiles."
            }
            AdaptationRule::LongTextChunk => {
                "Reading content above your target level is broken into smaller chunks."
            }
            AdaptationRule::DestructiveConfirm => {
                "A destructive control asks for confirmation before it acts."
            }
        }
    }
}

/// What a rule asks the renderer to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PlanActionKind {
    /// Cover a moving region with a static snapshot.
    Freeze,
    /// Present a dense menu as an icon grid.
    IconGrid,
    /// Break reading content into chunks.
    Chunk,
    /// Ask for confirmation before a destructive action.
    Confirm,
}

impl PlanActionKind {
    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            PlanActionKind::Freeze => "freeze",
            PlanActionKind::IconGrid => "icon_grid",
            PlanActionKind::Chunk => "chunk",
            PlanActionKind::Confirm => "confirm",
        }
    }

    /// The slot on a target that this action occupies.
    ///
    /// Two actions in the same group on the same target are alternatives; two
    /// actions in different groups are compatible.
    pub const fn conflict_group(self) -> ConflictGroup {
        match self {
            PlanActionKind::Freeze | PlanActionKind::IconGrid | PlanActionKind::Chunk => {
                ConflictGroup::Presentation
            }
            PlanActionKind::Confirm => ConflictGroup::Activation,
        }
    }
}

/// A slot on a target that only one action can occupy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConflictGroup {
    /// How a surface is presented. Freeze, grid, and chunk are alternatives.
    Presentation,
    /// Whether activation needs a check. Compatible with a presentation change.
    Activation,
}

/// The user's on/off switches for the built-in rules.
///
/// The default is every rule enabled. A disabled rule derives no action and
/// never wins a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RuleToggles(u8);

impl RuleToggles {
    /// Every rule enabled.
    pub const ALL_ENABLED: RuleToggles = RuleToggles(0);

    /// Every rule enabled.
    pub const fn new() -> Self {
        Self::ALL_ENABLED
    }

    /// The bit for a rule.
    const fn mask(rule: AdaptationRule) -> u8 {
        1 << (rule as u8)
    }

    /// Disables a rule.
    pub fn disable(&mut self, rule: AdaptationRule) {
        self.0 |= Self::mask(rule);
    }

    /// Enables a rule.
    pub fn enable(&mut self, rule: AdaptationRule) {
        self.0 &= !Self::mask(rule);
    }

    /// Sets whether a rule is enabled.
    pub fn set(&mut self, rule: AdaptationRule, enabled: bool) {
        if enabled {
            self.enable(rule);
        } else {
            self.disable(rule);
        }
    }

    /// Whether a rule is enabled.
    pub const fn is_enabled(self, rule: AdaptationRule) -> bool {
        self.0 & Self::mask(rule) == 0
    }

    /// Every enabled rule, in catalog order.
    pub fn enabled(self) -> impl Iterator<Item = AdaptationRule> {
        AdaptationRule::ALL
            .into_iter()
            .filter(move |rule| self.is_enabled(*rule))
    }

    /// The number of disabled rules.
    pub const fn disabled_count(self) -> usize {
        self.0.count_ones() as usize
    }
}

/// The profile switches and thresholds the rules read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleFlags {
    /// Freeze moving carousels.
    pub stop_motion: bool,
    /// Present dense menus as an icon grid.
    pub icon_grid: bool,
    /// Break reading content into chunks.
    pub chunking: bool,
    /// Ask for confirmation before destructive actions.
    pub confirm_actions: bool,
    /// The reading band the user wants.
    pub reading_target: ReadingLevel,
    /// Interactive elements that make a container dense.
    pub dense_menu_threshold: usize,
    /// Words a text must have before it can be chunked.
    pub min_text_words: usize,
}

impl Default for RuleFlags {
    fn default() -> Self {
        Self {
            stop_motion: true,
            icon_grid: true,
            chunking: true,
            confirm_actions: true,
            reading_target: ReadingLevel::Plain,
            dense_menu_threshold: DEFAULT_DENSE_MENU_THRESHOLD,
            min_text_words: DEFAULT_MIN_TEXT_WORDS,
        }
    }
}

/// What the engine reads for one frame.
#[derive(Debug, Clone, Copy)]
pub struct RuleContext<'a> {
    /// The merged screen graph.
    pub graph: &'a ScreenGraph,
    /// Media regions the perception layer reported.
    pub media: &'a [Media],
    /// The profile switches in force.
    pub flags: RuleFlags,
}

impl<'a> RuleContext<'a> {
    /// Creates a context.
    pub const fn new(graph: &'a ScreenGraph, media: &'a [Media], flags: RuleFlags) -> Self {
        Self {
            graph,
            media,
            flags,
        }
    }

    /// Creates a context with no media.
    pub fn from_graph(graph: &'a ScreenGraph, flags: RuleFlags) -> Self {
        Self {
            graph,
            media: &[],
            flags,
        }
    }
}

/// A fact the engine derives from the graph and the media list.
///
/// Every variant is content-free: ids, rectangles, kinds, word counts, and
/// reading bands. `Debug` on a fact never prints text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fact {
    /// A media region is moving.
    MovingMedia {
        /// The media rectangle.
        rect: Rect,
        /// The media kind.
        kind: MediaKind,
    },
    /// An element is a menu-like container holding at least the dense threshold.
    DenseContainer {
        /// The container element.
        id: ElementId,
        /// How many interactive elements it holds.
        interactive: usize,
    },
    /// An element carries measured reading content.
    TextLevel {
        /// The element.
        id: ElementId,
        /// Whitespace-separated word count.
        words: usize,
        /// The measured band.
        level: ReadingLevel,
    },
    /// An element is a destructive control.
    Destructive {
        /// The element.
        id: ElementId,
    },
}

/// One action the plan asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAction {
    /// What to do.
    pub kind: PlanActionKind,
    /// Which element it applies to.
    pub target: ElementId,
    /// Why it exists. Never `None` and never derived from the screen.
    pub rationale: Rationale,
}

impl PlanAction {
    /// Creates an action for a rule.
    fn new(rule: AdaptationRule, target: ElementId) -> Self {
        Self {
            kind: rule.action_kind(),
            target,
            rationale: Rationale::new(rule.id(), rule.rationale_text()),
        }
    }

    /// The rule that produced the action.
    pub const fn rule(&self) -> &RuleId {
        &self.rationale.rule
    }

    /// The rule's priority.
    pub fn priority(&self) -> u8 {
        priority_of(self.rule())
    }

    /// The slot on the target that this action occupies.
    pub const fn conflict_group(&self) -> ConflictGroup {
        self.kind.conflict_group()
    }
}

/// Why an action lost its conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupersedeReason {
    /// The winner has a higher priority.
    HigherPriority,
    /// Equal priority. In v1 every rule has a distinct priority, so this is one
    /// rule challenging itself on the same target and the first claim wins. If a
    /// future rule shares a priority, the rule id that sorts first wins.
    SamePriority,
}

/// An action that lost a conflict and was not planned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Superseded {
    /// The action that was dropped.
    pub action: PlanAction,
    /// The rule that won.
    pub winner: RuleId,
    /// Why it lost.
    pub reason: SupersedeReason,
}

/// The plan the engine produced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RulePlan {
    actions: Vec<PlanAction>,
    superseded: Vec<Superseded>,
}

impl RulePlan {
    /// The planned actions, highest priority first.
    pub fn actions(&self) -> &[PlanAction] {
        &self.actions
    }

    /// The actions that lost a conflict, in a stable order.
    pub fn superseded(&self) -> &[Superseded] {
        &self.superseded
    }

    /// Whether the plan has no actions.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// The number of planned actions.
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// The actions that target one element.
    pub fn actions_for(&self, target: ElementId) -> Vec<&PlanAction> {
        self.actions
            .iter()
            .filter(|action| action.target == target)
            .collect()
    }

    /// The rationale for one element, if any action targets it.
    pub fn rationale_for(&self, target: ElementId) -> Option<&Rationale> {
        self.actions
            .iter()
            .find(|action| action.target == target)
            .map(|action| &action.rationale)
    }
}

/// Applies the enabled rules to one frame.
#[derive(Debug, Clone)]
pub struct RuleEngine {
    toggles: RuleToggles,
    flags: RuleFlags,
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new(RuleToggles::new(), RuleFlags::default())
    }
}

impl RuleEngine {
    /// Creates an engine over a toggles set and a flags set.
    pub const fn new(toggles: RuleToggles, flags: RuleFlags) -> Self {
        Self { toggles, flags }
    }

    /// The toggles in force.
    pub const fn toggles(&self) -> RuleToggles {
        self.toggles
    }

    /// The flags in force.
    pub const fn flags(&self) -> &RuleFlags {
        &self.flags
    }

    /// Replaces the toggles.
    pub fn with_toggles(mut self, toggles: RuleToggles) -> Self {
        self.toggles = toggles;
        self
    }

    /// Replaces the flags.
    pub fn with_flags(mut self, flags: RuleFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Disables a rule.
    pub fn disable(&mut self, rule: AdaptationRule) {
        self.toggles.disable(rule);
    }

    /// Enables a rule.
    pub fn enable(&mut self, rule: AdaptationRule) {
        self.toggles.enable(rule);
    }

    /// Derives the facts for a frame.
    ///
    /// Pure: the same graph and media list always produce the same facts, in the
    /// same order.
    pub fn facts(&self, context: &RuleContext<'_>) -> Vec<Fact> {
        derive_facts(context)
    }

    /// Plans one frame.
    pub fn plan(&self, context: &RuleContext<'_>) -> RulePlan {
        evaluate(context, &self.toggles)
    }
}

/// A target a rule has already claimed.
struct Claim {
    /// The claimed element.
    target: ElementId,
    /// The slot it occupies.
    group: ConflictGroup,
    /// The winning rule.
    rule: RuleId,
    /// The winning rule's priority.
    priority: u8,
}

/// A container that holds at least the dense threshold.
struct DenseCandidate {
    /// The container element.
    id: ElementId,
    /// How many interactive elements it holds.
    interactive: usize,
    /// The container rectangle.
    rect: Rect,
}

/// The priority of a rule id, or `0` for an unknown id.
fn priority_of(rule: &RuleId) -> u8 {
    AdaptationRule::ALL
        .iter()
        .find(|candidate| candidate.id() == *rule)
        .map_or(0, |candidate| candidate.priority())
}

/// Whether a challenger displaces the claim it challenges.
///
/// A higher priority always wins. At equal priority the rule id that sorts
/// first wins, so the outcome never depends on evaluation order.
fn challenger_wins(challenger: AdaptationRule, claim: &Claim) -> bool {
    challenger.priority() > claim.priority
        || (challenger.priority() == claim.priority && challenger.name() < claim.rule.0.as_str())
}

/// The reason a loser lost to a winner.
fn reason_for(winner: u8, loser: u8) -> SupersedeReason {
    if winner == loser {
        SupersedeReason::SamePriority
    } else {
        SupersedeReason::HigherPriority
    }
}

/// Derives every fact from the context, in a stable order.
fn derive_facts(context: &RuleContext<'_>) -> Vec<Fact> {
    let mut facts = Vec::new();
    facts.extend(moving_media_facts(context.media));
    facts.extend(dense_container_facts(
        context.graph,
        context.flags.dense_menu_threshold,
    ));
    facts.extend(text_level_facts(context.graph));
    facts.extend(destructive_facts(context.graph));
    facts
}

/// A moving carousel is a fact; nothing else about the media list is.
fn moving_media_facts(media: &[Media]) -> Vec<Fact> {
    media
        .iter()
        .filter(|region| region.kind == MediaKind::Carousel && region.motion)
        .map(|region| Fact::MovingMedia {
            rect: region.rect,
            kind: region.kind,
        })
        .collect()
}

/// A menu-like container is a list, table, or dialog element that holds at
/// least `threshold` interactive elements.
///
/// The platform's `menu` role maps to one of those roles or to `Role::Unknown`;
/// v1 does not treat an unknown role as a container, so an unmapped menu is
/// missed rather than guessed at.
fn dense_container_facts(graph: &ScreenGraph, threshold: usize) -> Vec<Fact> {
    let centres: Vec<Point> = graph
        .elements
        .iter()
        .filter(|element| element.is_interactive())
        .map(|element| centre(element.rect))
        .collect();

    let mut candidates: Vec<DenseCandidate> = graph
        .elements
        .iter()
        .filter(|element| is_container_role(&element.role))
        .map(|element| DenseCandidate {
            id: element.id,
            interactive: centres
                .iter()
                .filter(|point| element.rect.contains(**point))
                .count(),
            rect: element.rect,
        })
        .filter(|candidate| candidate.interactive >= threshold)
        .collect();

    candidates.sort_by_key(|candidate| (candidate.rect.area(), candidate.id));

    // A container that fully contains a smaller qualifying container is the same
    // surface seen twice; keep the inner one.
    let mut kept: Vec<DenseCandidate> = Vec::new();
    for candidate in candidates {
        let nested = kept.iter().any(|inner| {
            candidate.rect.area() > inner.rect.area() && contains_rect(candidate.rect, inner.rect)
        });
        if !nested {
            kept.push(candidate);
        }
    }

    kept.into_iter()
        .map(|candidate| Fact::DenseContainer {
            id: candidate.id,
            interactive: candidate.interactive,
        })
        .collect()
}

/// Whether a role can hold a menu.
fn is_container_role(role: &Role) -> bool {
    matches!(role, Role::List | Role::Table | Role::Dialog)
}

/// Whether `outer` fully covers `inner`.
fn contains_rect(outer: Rect, inner: Rect) -> bool {
    outer
        .intersection(inner)
        .is_some_and(|overlap| overlap.area() == inner.area())
}

/// The reading band of every element that carries text.
///
/// A word is a whitespace-separated token that contains at least one letter or
/// digit, so punctuation-only strings are not reading content.
fn text_level_facts(graph: &ScreenGraph) -> Vec<Fact> {
    graph
        .elements
        .iter()
        .filter_map(|element| {
            let text = element.text.as_deref()?;
            let words = text
                .split_whitespace()
                .filter(|token| token.chars().any(|ch| ch.is_alphanumeric()))
                .count();
            if words == 0 {
                return None;
            }
            Some(Fact::TextLevel {
                id: element.id,
                words,
                level: reading_level(text),
            })
        })
        .collect()
}

/// Every button whose accessible name is a destructive term.
fn destructive_facts(graph: &ScreenGraph) -> Vec<Fact> {
    graph
        .elements
        .iter()
        .filter(|element| element.role == Role::Button)
        .filter(|element| {
            element
                .label
                .as_deref()
                .or(element.text.as_deref())
                .is_some_and(is_destructive)
        })
        .map(|element| Fact::Destructive { id: element.id })
        .collect()
}

/// Whether an accessible name contains a destructive term on a word boundary.
fn is_destructive(text: &str) -> bool {
    let name = text.to_lowercase();
    DESTRUCTIVE_TERMS
        .iter()
        .any(|term| contains_phrase(&name, term))
}

/// Whether `haystack` contains `needle` delimited by a non-alphanumeric edge.
fn contains_phrase(haystack: &str, needle: &str) -> bool {
    haystack.match_indices(needle).any(|(at, hit)| {
        let before = haystack[..at].chars().next_back();
        let after = haystack[at + hit.len()..].chars().next();
        before.is_none_or(|ch| !ch.is_ascii_alphanumeric())
            && after.is_none_or(|ch| !ch.is_ascii_alphanumeric())
    })
}

/// The actions one rule would take on a frame.
fn actions_for(
    rule: AdaptationRule,
    facts: &[Fact],
    graph: &ScreenGraph,
    flags: &RuleFlags,
) -> Vec<PlanAction> {
    match rule {
        AdaptationRule::CarouselIsDistraction => {
            if !flags.stop_motion {
                return Vec::new();
            }
            facts
                .iter()
                .filter_map(|fact| match fact {
                    Fact::MovingMedia { rect, .. } => best_element(*rect, &graph.elements)
                        .map(|target| PlanAction::new(rule, target)),
                    _ => None,
                })
                .collect()
        }
        AdaptationRule::DenseMenuToGrid => {
            if !flags.icon_grid {
                return Vec::new();
            }
            facts
                .iter()
                .filter_map(|fact| match fact {
                    Fact::DenseContainer { id, .. } => Some(PlanAction::new(rule, *id)),
                    _ => None,
                })
                .collect()
        }
        AdaptationRule::LongTextChunk => {
            if !flags.chunking {
                return Vec::new();
            }
            facts
                .iter()
                .filter_map(|fact| match fact {
                    Fact::TextLevel { id, words, level }
                        if *words >= flags.min_text_words && *level > flags.reading_target =>
                    {
                        Some(PlanAction::new(rule, *id))
                    }
                    _ => None,
                })
                .collect()
        }
        AdaptationRule::DestructiveConfirm => {
            if !flags.confirm_actions {
                return Vec::new();
            }
            facts
                .iter()
                .filter_map(|fact| match fact {
                    Fact::Destructive { id } => Some(PlanAction::new(rule, *id)),
                    _ => None,
                })
                .collect()
        }
    }
}

/// Offers one action to the working set and applies the outcome.
///
/// `claims` is the working set, `planned` its actions, and `superseded` collects
/// every offer that did not become an action.
fn offer(
    claims: &mut Vec<Claim>,
    planned: &mut Vec<PlanAction>,
    superseded: &mut Vec<Superseded>,
    action: PlanAction,
    challenger: AdaptationRule,
) {
    let group = action.conflict_group();
    let Some(index) = claims
        .iter()
        .position(|claim| claim.target == action.target && claim.group == group)
    else {
        claims.push(Claim {
            target: action.target,
            group,
            rule: challenger.id(),
            priority: challenger.priority(),
        });
        planned.push(action);
        return;
    };

    let claim = &claims[index];
    if challenger_wins(challenger, claim) {
        let reason = reason_for(challenger.priority(), claim.priority);
        let loser_index = planned
            .iter()
            .position(|claimed| {
                claimed.target == action.target && claimed.conflict_group() == group
            })
            .expect("a claim always has a planned action");
        let loser = planned.remove(loser_index);
        claims[index] = Claim {
            target: action.target,
            group,
            rule: challenger.id(),
            priority: challenger.priority(),
        };
        superseded.push(Superseded {
            action: loser,
            winner: challenger.id(),
            reason,
        });
        planned.push(action);
    } else {
        superseded.push(Superseded {
            action,
            winner: claim.rule.clone(),
            reason: reason_for(claim.priority, challenger.priority()),
        });
    }
}

/// Evaluates the enabled rules and resolves their conflicts.
///
/// Rules run in catalog order. Each rule offers its actions to a working set of
/// claims; a later offer for a target and group a higher-priority rule already
/// holds replaces the claim, and every replaced or refused offer is recorded in
/// `superseded`. The working set is the engine's forward-chaining memory: a
/// conclusion an earlier rule reached constrains every rule after it.
fn evaluate(context: &RuleContext<'_>, toggles: &RuleToggles) -> RulePlan {
    let facts = derive_facts(context);
    let flags = context.flags;
    let mut plan = RulePlan::default();
    let mut claims: Vec<Claim> = Vec::new();

    for rule in AdaptationRule::ALL {
        if !toggles.is_enabled(rule) {
            continue;
        }
        for action in actions_for(rule, &facts, context.graph, &flags) {
            offer(
                &mut claims,
                &mut plan.actions,
                &mut plan.superseded,
                action,
                rule,
            );
        }
    }

    sort_plan(&mut plan);
    plan
}

/// Sorts a plan's actions and superseded offers into a stable order.
fn sort_plan(plan: &mut RulePlan) {
    plan.actions.sort_by(|a, b| {
        b.priority()
            .cmp(&a.priority())
            .then_with(|| a.rule().cmp(b.rule()))
            .then_with(|| a.target.cmp(&b.target))
    });
    plan.superseded.sort_by(|a, b| {
        b.action
            .priority()
            .cmp(&a.action.priority())
            .then_with(|| a.action.rule().cmp(b.action.rule()))
            .then_with(|| a.action.target.cmp(&b.action.target))
    });
}

/// The element a media rectangle sits on: the one it overlaps most.
///
/// Ties go to the smaller element, then to the smaller id, so the choice is
/// deterministic. A media region that overlaps no element has no target.
fn best_element(rect: Rect, elements: &[Element]) -> Option<ElementId> {
    elements
        .iter()
        .filter_map(|element| {
            element
                .rect
                .intersection(rect)
                .map(|overlap| (element, overlap.area()))
        })
        .max_by_key(|(element, area)| (*area, Reverse(element.rect.area()), Reverse(element.id)))
        .map(|(element, _)| element.id)
}

/// The centre of a rectangle, widened to `i64` before summing.
fn centre(rect: Rect) -> Point {
    Point::new(
        ((rect.x as i64 + rect.right() as i64) / 2) as i32,
        ((rect.y as i64 + rect.bottom() as i64) / 2) as i32,
    )
}

#[cfg(test)]
mod tests;
