//! One-step mode with visible answers.
//!
//! A multi-step flow is walked one step at a time. Exactly one step is visible,
//! a progress indicator reports the position, and every value the user enters
//! stays in a persistent "Your answers" panel. Back is always available.
//!
//! Four rules hold:
//!
//! - **Only one step is visible.** [`OneStepSession::visible`] returns at most
//!   one step, and [`OneStepSession::visible_count`] is 0 or 1.
//! - **Answers persist.** [`Answers`] keeps every value keyed by [`StepId`], so
//!   moving back and forward does not lose an entry.
//! - **Back is always available.** [`OneStepSession::back`] is callable at any
//!   time; at the first step it reports `false` and changes nothing.
//! - **Values are redacted.** [`Answers`] and [`Answer`] implement `Debug` by
//!   hand and print `<redacted>` for every value, so a log line cannot print
//!   what the user typed.

use std::collections::BTreeMap;
use std::fmt;

/// A step in a flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StepId(pub u64);

impl StepId {
    /// The raw numeric value.
    pub fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for StepId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StepId({})", self.0)
    }
}

/// What kind of answer a step takes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StepKind {
    /// A free-text answer.
    #[default]
    Text,
    /// A single choice from a list.
    Choice,
    /// A yes or no decision.
    Confirm,
}

impl StepKind {
    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Choice => "choice",
            Self::Confirm => "confirm",
        }
    }

    /// The human-readable label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Text => "answer",
            Self::Choice => "choice",
            Self::Confirm => "yes or no",
        }
    }

    /// Resolves a name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "text" => Some(Self::Text),
            "choice" => Some(Self::Choice),
            "confirm" => Some(Self::Confirm),
            _ => None,
        }
    }
}

/// One step of a flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    id: StepId,
    kind: StepKind,
    prompt: String,
    required: bool,
    choices: Vec<String>,
}

impl Step {
    /// Creates a step.
    pub fn new(id: StepId, kind: StepKind, prompt: impl Into<String>, required: bool) -> Self {
        Self {
            id,
            kind,
            prompt: prompt.into(),
            required,
            choices: Vec::new(),
        }
    }

    /// Creates a choice step.
    pub fn choice(
        id: StepId,
        prompt: impl Into<String>,
        required: bool,
        choices: Vec<String>,
    ) -> Self {
        Self {
            id,
            kind: StepKind::Choice,
            prompt: prompt.into(),
            required,
            choices,
        }
    }

    /// The step's id.
    pub const fn id(&self) -> StepId {
        self.id
    }

    /// The kind of answer it takes.
    pub const fn kind(&self) -> StepKind {
        self.kind
    }

    /// The prompt shown to the user.
    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    /// Whether an answer is required before the flow can finish.
    pub const fn is_required(&self) -> bool {
        self.required
    }

    /// The choices offered, empty unless the kind is [`StepKind::Choice`].
    pub fn choices(&self) -> &[String] {
        &self.choices
    }

    /// Whether a value is one of the offered choices.
    ///
    /// A step with no choices accepts any value.
    pub fn accepts(&self, value: &str) -> bool {
        self.choices.is_empty() || self.choices.iter().any(|choice| choice.as_str() == value)
    }
}

/// One answer in the "Your answers" panel.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Answer {
    step_id: StepId,
    value: String,
}

impl Answer {
    /// Creates an answer.
    pub fn new(step_id: StepId, value: impl Into<String>) -> Self {
        Self {
            step_id,
            value: value.into(),
        }
    }

    /// The step the value belongs to.
    pub const fn step_id(&self) -> StepId {
        self.step_id
    }

    /// The value.
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Debug for Answer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Answer")
            .field("step_id", &self.step_id)
            .field("value", &"<redacted>")
            .finish()
    }
}

/// The persistent "Your answers" panel.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Answers {
    values: BTreeMap<StepId, String>,
}

impl Answers {
    /// Creates an empty panel.
    pub fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    /// Records a value for a step, replacing any previous one.
    pub fn set(&mut self, step_id: StepId, value: impl Into<String>) {
        self.values.insert(step_id, value.into());
    }

    /// The value recorded for a step.
    pub fn get(&self, step_id: StepId) -> Option<&str> {
        self.values.get(&step_id).map(String::as_str)
    }

    /// Whether a step has an answer.
    pub fn contains(&self, step_id: StepId) -> bool {
        self.values.contains_key(&step_id)
    }

    /// How many answers are recorded.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether nothing is recorded.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Removes one answer.
    pub fn remove(&mut self, step_id: StepId) -> bool {
        self.values.remove(&step_id).is_some()
    }

    /// Clears the panel.
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// The answers in [`StepId`] order.
    pub fn entries(&self) -> Vec<Answer> {
        self.values
            .iter()
            .map(|(step_id, value)| Answer::new(*step_id, value.clone()))
            .collect()
    }

    /// The steps that have an answer, in [`StepId`] order.
    pub fn step_ids(&self) -> Vec<StepId> {
        self.values.keys().copied().collect()
    }

    /// Merges another panel, keeping this panel's value on a conflict.
    pub fn merge(&mut self, other: &Self) {
        for (step_id, value) in &other.values {
            self.values.entry(*step_id).or_insert(value.clone());
        }
    }

    /// Merges another panel, taking the other panel's value on a conflict.
    pub fn merge_overriding(&mut self, other: &Self) {
        for (step_id, value) in &other.values {
            self.values.insert(*step_id, value.clone());
        }
    }
}

impl fmt::Debug for Answers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Answers")
            .field("count", &self.values.len())
            .field("values", &"<redacted>")
            .finish()
    }
}

/// The position within a flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    current: usize,
    total: usize,
}

impl Progress {
    /// Creates a progress.
    ///
    /// `current` is the zero-based index of the visible step and `total` is the
    /// number of steps.
    pub const fn new(current: usize, total: usize) -> Self {
        Self { current, total }
    }

    /// The zero-based index of the visible step.
    pub const fn current(self) -> usize {
        self.current
    }

    /// The number of steps.
    pub const fn total(self) -> usize {
        self.total
    }

    /// The one-based step number, or 0 when the flow is empty.
    pub const fn step_number(self) -> usize {
        if self.total == 0 {
            0
        } else {
            self.current + 1
        }
    }

    /// The fraction of the flow reached, `0.0..=1.0`.
    ///
    /// The visible step counts as reached, so the first step of three is
    /// `1/3` and the last is `1.0`. An empty flow is `0.0`.
    pub fn ratio(self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.current + 1) as f32 / self.total as f32
        }
    }

    /// The percentage of the flow completed, `0.0..=100.0`.
    pub fn percent(self) -> f32 {
        self.ratio() * 100.0
    }

    /// Whether the visible step is the first.
    pub const fn is_first(self) -> bool {
        self.current == 0
    }

    /// Whether the visible step is the last.
    pub const fn is_last(self) -> bool {
        self.total != 0 && self.current + 1 == self.total
    }

    /// Whether the flow is empty.
    pub const fn is_empty(self) -> bool {
        self.total == 0
    }

    /// The indicator text, for example `Step 2 of 5`.
    pub fn label(self) -> String {
        format!("Step {} of {}", self.step_number(), self.total)
    }
}

/// An ordered flow of steps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Flow {
    steps: Vec<Step>,
}

impl Flow {
    /// Creates a flow from steps.
    pub fn new(steps: Vec<Step>) -> Self {
        Self { steps }
    }

    /// Creates an empty flow.
    pub fn empty() -> Self {
        Self { steps: Vec::new() }
    }

    /// Appends a step.
    pub fn push(&mut self, step: Step) {
        self.steps.push(step);
    }

    /// The steps in order.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// How many steps the flow has.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Whether the flow has no steps.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The step at an index.
    pub fn get(&self, index: usize) -> Option<&Step> {
        self.steps.get(index)
    }

    /// The index of a step id.
    pub fn index_of(&self, id: StepId) -> Option<usize> {
        self.steps.iter().position(|step| step.id == id)
    }

    /// The first step.
    pub fn first(&self) -> Option<&Step> {
        self.steps.first()
    }

    /// The last step.
    pub fn last(&self) -> Option<&Step> {
        self.steps.last()
    }

    /// Whether an index is within the flow.
    pub fn is_valid_index(&self, index: usize) -> bool {
        index < self.steps.len()
    }

    /// The number of required steps.
    pub fn required_count(&self) -> usize {
        self.steps.iter().filter(|step| step.required).count()
    }
}

/// A session that walks a [`Flow`] one step at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneStepSession {
    flow: Flow,
    index: usize,
    answers: Answers,
    finished: bool,
}

impl OneStepSession {
    /// Creates a session at the first step.
    pub fn new(flow: Flow) -> Self {
        Self {
            flow,
            index: 0,
            answers: Answers::new(),
            finished: false,
        }
    }

    /// The flow.
    pub fn flow(&self) -> &Flow {
        &self.flow
    }

    /// The index of the visible step.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The visible step, or `None` when the flow is empty or finished.
    pub fn current(&self) -> Option<&Step> {
        if self.finished {
            return None;
        }
        self.flow.get(self.index)
    }

    /// The visible step.
    ///
    /// This is the same step as [`current`](Self::current); the name states the
    /// one-step rule directly.
    pub fn visible(&self) -> Option<&Step> {
        self.current()
    }

    /// How many steps are visible: 0 or 1.
    pub fn visible_count(&self) -> usize {
        usize::from(self.visible().is_some())
    }

    /// Whether the visible step is the first.
    pub fn is_first(&self) -> bool {
        self.index == 0
    }

    /// Whether the visible step is the last.
    pub fn is_last(&self) -> bool {
        self.flow.is_valid_index(self.index) && self.index + 1 == self.flow.len()
    }

    /// Whether back would move.
    pub fn can_back(&self) -> bool {
        self.index > 0
    }

    /// Whether advancing would move.
    pub fn can_advance(&self) -> bool {
        !self.finished && self.flow.is_valid_index(self.index) && self.index + 1 < self.flow.len()
    }

    /// Moves to the previous step.
    ///
    /// Back is always callable. At the first step it reports `false` and
    /// changes nothing.
    pub fn back(&mut self) -> bool {
        if !self.can_back() {
            return false;
        }
        self.finished = false;
        self.index -= 1;
        true
    }

    /// Moves to the next step.
    ///
    /// At the last step it reports `false` and changes nothing.
    pub fn advance(&mut self) -> bool {
        if !self.can_advance() {
            return false;
        }
        self.index += 1;
        true
    }

    /// Jumps to a step by id.
    pub fn jump_to(&mut self, id: StepId) -> bool {
        match self.flow.index_of(id) {
            Some(index) => {
                self.index = index;
                self.finished = false;
                true
            }
            None => false,
        }
    }

    /// Answers the visible step.
    ///
    /// Returns `false` when there is no visible step or the value is not
    /// accepted by the step.
    pub fn answer(&mut self, value: impl Into<String>) -> bool {
        let value = value.into();
        if self.finished {
            return false;
        }
        let step = match self.flow.get(self.index) {
            Some(step) => step,
            None => return false,
        };
        if !step.accepts(&value) {
            return false;
        }
        let id = step.id;
        self.answers.set(id, value);
        true
    }

    /// Answers a step by id, whether or not it is visible.
    pub fn answer_step(&mut self, id: StepId, value: impl Into<String>) -> bool {
        let value = value.into();
        match self.flow.steps.iter().find(|step| step.id == id) {
            Some(step) if step.accepts(&value) => {
                self.answers.set(id, value);
                true
            }
            _ => false,
        }
    }

    /// The "Your answers" panel.
    pub fn answers(&self) -> &Answers {
        &self.answers
    }

    /// The progress indicator.
    pub fn progress(&self) -> Progress {
        Progress::new(self.index, self.flow.len())
    }

    /// Whether every required step has an answer.
    pub fn is_complete(&self) -> bool {
        self.flow
            .steps()
            .iter()
            .filter(|step| step.required)
            .all(|step| self.answers.contains(step.id))
    }

    /// Whether every step has an answer.
    pub fn is_fully_answered(&self) -> bool {
        self.flow
            .steps()
            .iter()
            .all(|step| self.answers.contains(step.id))
    }

    /// Marks the flow finished when it is complete.
    ///
    /// Returns whether the flow is now finished.
    pub fn finish(&mut self) -> bool {
        if self.is_complete() {
            self.finished = true;
            true
        } else {
            false
        }
    }

    /// Whether the flow is finished.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Returns to the first step and clears the answers.
    pub fn restart(&mut self) {
        self.index = 0;
        self.finished = false;
        self.answers.clear();
    }
}

#[cfg(test)]
mod tests;
