//! Meaning-preservation checker.
//!
//! A simplification that drops a named entity, a number, a negation, or a
//! comparative changes what the reader understands. The checker extracts those
//! four categories from the original and from the candidate, and rejects the
//! candidate when any of them is missing. See
//! [`../../../docs/MEANING.md`](../../../docs/MEANING.md).
//!
//! Four rules hold:
//!
//! - **Recall first.** A missing entity, number, negation, or comparative is a
//!   rejection, even when the sentence still reads well. The checker would
//!   rather fall back to the original than show a rewrite that lost a fact.
//! - **Comparison is by multiset.** Repeated facts are counted, so "5 items and
//!   5 more" cannot be reduced to "5 items" and still pass.
//! - **Rejection is counted, not quoted.** [`MeaningReport`] holds counts in
//!   four categories; [`RejectionLog`] accumulates them. No token from the
//!   screen is stored or logged.
//! - **The original is the fallback.** [`MeaningChecker::rewrite`] returns the
//!   candidate only when [`MeaningDecision::Preserved`]; otherwise it returns
//!   the original.

use std::fmt;

/// Why a candidate was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RejectionReason {
    /// A named entity from the original is absent.
    MissingEntity,
    /// A number from the original is absent.
    MissingNumber,
    /// A negation from the original is absent.
    MissingNegation,
    /// A comparative from the original is absent.
    MissingComparative,
}

impl RejectionReason {
    /// Every reason, in reporting order.
    pub const ALL: [RejectionReason; 4] = [
        Self::MissingEntity,
        Self::MissingNumber,
        Self::MissingNegation,
        Self::MissingComparative,
    ];

    /// The stable name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingEntity => "missing-entity",
            Self::MissingNumber => "missing-number",
            Self::MissingNegation => "missing-negation",
            Self::MissingComparative => "missing-comparative",
        }
    }

    /// The human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::MissingEntity => "named entity",
            Self::MissingNumber => "number",
            Self::MissingNegation => "negation",
            Self::MissingComparative => "comparative",
        }
    }

    /// The plural label used in a summary.
    pub fn plural(self) -> &'static str {
        match self {
            Self::MissingEntity => "named entities",
            Self::MissingNumber => "numbers",
            Self::MissingNegation => "negations",
            Self::MissingComparative => "comparatives",
        }
    }
}

impl fmt::Display for RejectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Counts in the four meaning categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CategoryCounts {
    /// Named entities.
    pub entities: usize,
    /// Numbers.
    pub numbers: usize,
    /// Negations.
    pub negations: usize,
    /// Comparatives.
    pub comparatives: usize,
}

impl CategoryCounts {
    /// The count for one reason.
    pub const fn get(self, reason: RejectionReason) -> usize {
        match reason {
            RejectionReason::MissingEntity => self.entities,
            RejectionReason::MissingNumber => self.numbers,
            RejectionReason::MissingNegation => self.negations,
            RejectionReason::MissingComparative => self.comparatives,
        }
    }

    /// Sets the count for one reason.
    pub fn set(&mut self, reason: RejectionReason, value: usize) {
        match reason {
            RejectionReason::MissingEntity => self.entities = value,
            RejectionReason::MissingNumber => self.numbers = value,
            RejectionReason::MissingNegation => self.negations = value,
            RejectionReason::MissingComparative => self.comparatives = value,
        }
    }

    /// The total across the four categories.
    pub const fn total(self) -> usize {
        self.entities + self.numbers + self.negations + self.comparatives
    }

    /// Whether every category is zero.
    pub const fn is_empty(self) -> bool {
        self.total() == 0
    }

    /// The reasons whose count is non-zero, in [`RejectionReason::ALL`] order.
    pub fn reasons(self) -> Vec<RejectionReason> {
        RejectionReason::ALL
            .into_iter()
            .filter(|reason| self.get(*reason) > 0)
            .collect()
    }
}

/// What the checker found, as counts only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeaningReport {
    original: CategoryCounts,
    simplified: CategoryCounts,
    missing: CategoryCounts,
}

impl MeaningReport {
    /// Creates a report.
    pub const fn new(
        original: CategoryCounts,
        simplified: CategoryCounts,
        missing: CategoryCounts,
    ) -> Self {
        Self {
            original,
            simplified,
            missing,
        }
    }

    /// The counts in the original text.
    pub const fn original(&self) -> CategoryCounts {
        self.original
    }

    /// The counts in the candidate text.
    pub const fn simplified(&self) -> CategoryCounts {
        self.simplified
    }

    /// The counts that were missing from the candidate.
    pub const fn missing(&self) -> CategoryCounts {
        self.missing
    }

    /// The reasons the candidate was rejected for, in a stable order.
    pub fn reasons(&self) -> Vec<RejectionReason> {
        self.missing.reasons()
    }

    /// The fraction of original facts the candidate kept, in `0.0..=1.0`.
    ///
    /// A text with no facts to preserve scores `1.0`.
    pub fn recall(&self) -> f32 {
        let total = self.original.total();
        if total == 0 {
            return 1.0;
        }
        1.0 - (self.missing.total() as f32 / total as f32)
    }

    /// A content-free summary such as `missing 1 number, 2 negations`.
    pub fn describe(&self) -> String {
        let parts: Vec<String> = self
            .missing
            .reasons()
            .into_iter()
            .map(|reason| {
                let count = self.missing.get(reason);
                let noun = if count == 1 {
                    reason.label().to_string()
                } else {
                    reason.plural().to_string()
                };
                format!("{count} {noun}")
            })
            .collect();
        format!("missing {}", parts.join(", "))
    }
}

/// The result of comparing an original with a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeaningDecision {
    /// The candidate kept every fact.
    Preserved,
    /// A fact was lost; the caller must fall back.
    Rejected(MeaningReport),
}

impl MeaningDecision {
    /// Whether the candidate was preserved.
    pub const fn is_preserved(&self) -> bool {
        matches!(self, Self::Preserved)
    }

    /// Whether the candidate was rejected.
    pub const fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected(_))
    }

    /// The report, when the candidate was rejected.
    pub const fn report(&self) -> Option<&MeaningReport> {
        match self {
            Self::Preserved => None,
            Self::Rejected(report) => Some(report),
        }
    }

    /// Records every rejection reason in a log.
    pub fn record(&self, log: &mut RejectionLog) {
        if let Self::Rejected(report) = self {
            log.record_report(report);
        }
    }
}

/// What a caller may show after a rewrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeaningSimplification {
    /// The rewrite kept every fact.
    Simplified(String),
    /// The rewrite was rejected; show the original.
    Original(String),
}

impl MeaningSimplification {
    /// The text to show.
    pub fn text(&self) -> &str {
        match self {
            Self::Simplified(text) | Self::Original(text) => text,
        }
    }

    /// Whether this is the untouched original.
    pub fn is_original(&self) -> bool {
        matches!(self, Self::Original(_))
    }
}

/// A count-only log of rejections.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RejectionLog {
    counts: [usize; 4],
    total: usize,
}

impl RejectionLog {
    /// Records one rejection.
    pub fn record(&mut self, reason: RejectionReason) {
        let index = reason_index(reason);
        self.counts[index] += 1;
        self.total += 1;
    }

    /// Records every reason in a report.
    pub fn record_report(&mut self, report: &MeaningReport) {
        for reason in report.reasons() {
            for _ in 0..report.missing.get(reason) {
                self.record(reason);
            }
        }
    }

    /// How many times a reason was recorded.
    pub fn count(&self, reason: RejectionReason) -> usize {
        self.counts[reason_index(reason)]
    }

    /// The total number of recorded rejections.
    pub fn total(&self) -> usize {
        self.total
    }

    /// Whether nothing was recorded.
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// Adds another log's counts to this one.
    pub fn merge(&mut self, other: &Self) {
        for (index, count) in other.counts.iter().enumerate() {
            self.counts[index] += count;
        }
        self.total += other.total;
    }

    /// The counts as categories.
    pub fn counts(&self) -> CategoryCounts {
        let mut counts = CategoryCounts::default();
        for reason in RejectionReason::ALL {
            counts.set(reason, self.count(reason));
        }
        counts
    }
}

/// The checker.
#[derive(Debug, Clone, Copy, Default)]
pub struct MeaningChecker;

impl MeaningChecker {
    /// Creates a checker.
    pub const fn new() -> Self {
        Self
    }

    /// Compares an original with a candidate.
    pub fn check(&self, original: &str, simplified: &str) -> MeaningDecision {
        let original_entities = extract_entities(original);
        let simplified_entities = extract_entities(simplified);
        let original_numbers = extract_numbers(original);
        let simplified_numbers = extract_numbers(simplified);
        let original_negations = extract_negations(original);
        let simplified_negations = extract_negations(simplified);
        let original_comparatives = extract_comparatives(original);
        let simplified_comparatives = extract_comparatives(simplified);

        let missing = CategoryCounts {
            entities: missing_count(&original_entities, &simplified_entities),
            numbers: missing_count(&original_numbers, &simplified_numbers),
            negations: missing_count(&original_negations, &simplified_negations),
            comparatives: missing_count(&original_comparatives, &simplified_comparatives),
        };

        if missing.is_empty() {
            return MeaningDecision::Preserved;
        }

        let report = MeaningReport::new(
            CategoryCounts {
                entities: original_entities.len(),
                numbers: original_numbers.len(),
                negations: original_negations.len(),
                comparatives: original_comparatives.len(),
            },
            CategoryCounts {
                entities: simplified_entities.len(),
                numbers: simplified_numbers.len(),
                negations: simplified_negations.len(),
                comparatives: simplified_comparatives.len(),
            },
            missing,
        );
        MeaningDecision::Rejected(report)
    }

    /// The report for a rejected pair, or `None` when it was preserved.
    pub fn report(&self, original: &str, simplified: &str) -> Option<MeaningReport> {
        match self.check(original, simplified) {
            MeaningDecision::Preserved => None,
            MeaningDecision::Rejected(report) => Some(report),
        }
    }

    /// The fraction of facts the candidate kept.
    pub fn recall(&self, original: &str, simplified: &str) -> f32 {
        match self.report(original, simplified) {
            Some(report) => report.recall(),
            None => 1.0,
        }
    }

    /// Decides what to show after a rewrite.
    ///
    /// The candidate is shown only when it is non-empty and preserved.
    pub fn rewrite(&self, original: &str, candidate: &str) -> MeaningSimplification {
        if candidate.trim().is_empty() {
            return MeaningSimplification::Original(original.to_string());
        }
        match self.check(original, candidate) {
            MeaningDecision::Preserved => MeaningSimplification::Simplified(candidate.to_string()),
            MeaningDecision::Rejected(_) => MeaningSimplification::Original(original.to_string()),
        }
    }
}

/// Extracts named entities: runs of capitalized words, trimmed of stopwords.
pub fn extract_entities(text: &str) -> Vec<String> {
    let mut entities = Vec::new();
    let mut run: Vec<&str> = Vec::new();
    let mut pending: Vec<&str> = Vec::new();

    for word in words(text) {
        if is_capitalized(word) {
            run.append(&mut pending);
            pending.clear();
            run.push(word);
        } else if !run.is_empty() && is_internal_word(word) {
            pending.push(word);
        } else if !run.is_empty() {
            push_entity(&mut entities, &run);
            run.clear();
            pending.clear();
        } else {
            pending.clear();
        }
    }
    if !run.is_empty() {
        push_entity(&mut entities, &run);
    }
    entities
}

/// Trims stopwords from a run and records the entity.
fn push_entity(entities: &mut Vec<String>, run: &[&str]) {
    let start = run
        .iter()
        .position(|word| !is_stopword(word))
        .unwrap_or(run.len());
    let end = run
        .iter()
        .rposition(|word| !is_stopword(word))
        .map_or(start, |last| last + 1);
    if start >= end {
        return;
    }
    let entity = run[start..end].join(" ").to_lowercase();
    if !entity.is_empty() {
        entities.push(entity);
    }
}

/// Whether a word starts with an uppercase letter.
fn is_capitalized(word: &str) -> bool {
    word.chars().next().is_some_and(char::is_uppercase)
}

/// Whether a lowercase word may appear inside a name.
fn is_internal_word(word: &str) -> bool {
    matches!(
        word,
        "of" | "the" | "and" | "for" | "de" | "van" | "von" | "del" | "la" | "le" | "du"
    )
}

/// Whether a word is too common to be part of a name.
fn is_stopword(word: &str) -> bool {
    matches!(
        word.to_lowercase().as_str(),
        "a" | "an"
            | "all"
            | "also"
            | "although"
            | "and"
            | "another"
            | "any"
            | "are"
            | "as"
            | "at"
            | "available"
            | "bad"
            | "be"
            | "because"
            | "been"
            | "before"
            | "being"
            | "best"
            | "better"
            | "but"
            | "by"
            | "can"
            | "cannot"
            | "closed"
            | "could"
            | "did"
            | "different"
            | "do"
            | "does"
            | "down"
            | "dr"
            | "each"
            | "equal"
            | "even"
            | "every"
            | "first"
            | "for"
            | "free"
            | "from"
            | "good"
            | "had"
            | "has"
            | "have"
            | "he"
            | "hello"
            | "her"
            | "here"
            | "hey"
            | "hi"
            | "higher"
            | "his"
            | "how"
            | "however"
            | "i"
            | "if"
            | "important"
            | "in"
            | "is"
            | "it"
            | "its"
            | "just"
            | "last"
            | "least"
            | "less"
            | "lower"
            | "madam"
            | "may"
            | "might"
            | "more"
            | "most"
            | "mr"
            | "mrs"
            | "ms"
            | "much"
            | "must"
            | "my"
            | "new"
            | "next"
            | "no"
            | "note"
            | "now"
            | "of"
            | "ok"
            | "okay"
            | "old"
            | "on"
            | "only"
            | "open"
            | "or"
            | "other"
            | "our"
            | "please"
            | "previous"
            | "prof"
            | "required"
            | "same"
            | "shall"
            | "she"
            | "should"
            | "since"
            | "sir"
            | "so"
            | "some"
            | "still"
            | "such"
            | "than"
            | "thanks"
            | "that"
            | "the"
            | "their"
            | "them"
            | "then"
            | "there"
            | "these"
            | "they"
            | "this"
            | "those"
            | "though"
            | "to"
            | "today"
            | "too"
            | "tomorrow"
            | "until"
            | "unless"
            | "up"
            | "very"
            | "was"
            | "we"
            | "welcome"
            | "were"
            | "what"
            | "when"
            | "where"
            | "which"
            | "while"
            | "who"
            | "why"
            | "will"
            | "with"
            | "would"
            | "worst"
            | "yes"
            | "yesterday"
            | "you"
            | "your"
    )
}

/// Extracts numbers, normalized for comparison.
pub fn extract_numbers(text: &str) -> Vec<String> {
    let mut numbers = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        let digit_start = byte.is_ascii_digit()
            || (byte == b'-' && index + 1 < bytes.len() && bytes[index + 1].is_ascii_digit());
        if !digit_start {
            index += 1;
            continue;
        }
        let start = index;
        if byte == b'-' {
            index += 1;
        }
        while index < bytes.len()
            && (bytes[index].is_ascii_digit() || matches!(bytes[index], b',' | b'_' | b'.'))
        {
            index += 1;
        }
        if index < bytes.len() && bytes[index] == b'%' {
            index += 1;
        }
        let token = &text[start..index];
        let normalized = normalize_number(token);
        if !normalized.is_empty() {
            numbers.push(normalized);
        }
    }
    numbers
}

/// Removes separators and trailing zeros so `1,200`, `1200`, and `1200.00`
/// compare equal.
fn normalize_number(token: &str) -> String {
    let mut text = token.replace([',', '_', '%'], "");
    text = text.trim_end_matches('.').to_string();
    if text.contains('.') {
        text = text.trim_end_matches('0').to_string();
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

/// Extracts negation words and phrases in text order.
pub fn extract_negations(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut found = Vec::new();
    for word in words(&lower) {
        if NEGATIONS.contains(&word) {
            found.push((*word).to_string());
        }
    }
    for phrase in NEGATIONS {
        if phrase.contains(' ') {
            for _ in 0..lower.matches(phrase).count() {
                found.push((*phrase).to_string());
            }
        }
    }
    found
}

/// Extracts comparative and superlative words in text order.
pub fn extract_comparatives(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut found = Vec::new();
    for word in words(&lower) {
        if COMPARATIVES.contains(&word) {
            found.push((*word).to_string());
        }
    }
    found
}

/// Splits a text into alphanumeric words, keeping apostrophes.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|word| !word.is_empty())
}

/// Counts original tokens with no counterpart in the candidate.
///
/// The comparison is a multiset: each candidate token can satisfy one original
/// token, so a repeated fact must appear twice.
fn missing_count(original: &[String], candidate: &[String]) -> usize {
    let mut pool: Vec<&str> = candidate.iter().map(String::as_str).collect();
    let mut missing = 0;
    for token in original {
        match pool.iter().position(|other| *other == token.as_str()) {
            Some(index) => {
                pool.remove(index);
            }
            None => missing += 1,
        }
    }
    missing
}

/// The index of a reason in [`RejectionReason::ALL`].
const fn reason_index(reason: RejectionReason) -> usize {
    match reason {
        RejectionReason::MissingEntity => 0,
        RejectionReason::MissingNumber => 1,
        RejectionReason::MissingNegation => 2,
        RejectionReason::MissingComparative => 3,
    }
}

/// Negation words and phrases.
const NEGATIONS: &[&str] = &[
    "not",
    "no",
    "never",
    "none",
    "nothing",
    "nobody",
    "no one",
    "nowhere",
    "neither",
    "nor",
    "cannot",
    "can't",
    "won't",
    "don't",
    "doesn't",
    "didn't",
    "isn't",
    "aren't",
    "wasn't",
    "weren't",
    "hasn't",
    "haven't",
    "hadn't",
    "shouldn't",
    "wouldn't",
    "couldn't",
    "mustn't",
    "without",
    "hardly",
    "barely",
    "scarcely",
    "rarely",
];

/// Comparative and superlative words.
const COMPARATIVES: &[&str] = &[
    "more",
    "less",
    "fewer",
    "than",
    "better",
    "worse",
    "best",
    "worst",
    "greater",
    "larger",
    "smaller",
    "higher",
    "lower",
    "bigger",
    "longer",
    "shorter",
    "faster",
    "slower",
    "earlier",
    "later",
    "before",
    "after",
    "equal",
    "same",
    "different",
    "most",
    "least",
    "similar",
    "unlike",
    "versus",
    "vs",
    "above",
    "below",
    "increase",
    "decrease",
    "reduce",
    "raise",
    "grow",
    "shrink",
    "improve",
    "decline",
    "compared",
    "comparison",
    "majority",
    "minority",
    "half",
    "double",
    "twice",
    "triple",
];

/// Pairs that must be preserved.
pub fn preserved_pairs() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "Your deductible is $1,200 per year.",
            "You pay $1,200 each year.",
        ),
        (
            "The appointment is on 16 November 2026.",
            "The appointment is on 16 November 2026.",
        ),
        ("Do not cancel the policy.", "Do not cancel the policy."),
        (
            "The price is higher than last year.",
            "The price is higher than last year.",
        ),
        ("You have 3 new messages.", "You have 3 new messages."),
        (
            "The bank is closed on Sunday.",
            "The bank is shut on Sunday.",
        ),
        ("Neither option is free.", "Neither choice is free."),
        ("Your balance is $0.00.", "Your balance is $0."),
        ("The offer ends in 2 days.", "The offer ends in 2 days."),
        ("We cannot refund this item.", "We cannot refund this item."),
    ]
}

/// Pairs that must be rejected, one per category and a few combined cases.
pub fn adversarial_pairs() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "Your deductible is $1,200 per year.",
            "Your deductible is $2,100 per year.",
        ),
        ("Do not cancel the policy.", "Cancel the policy."),
        (
            "The price is higher than last year.",
            "The price is lower than last year.",
        ),
        (
            "Apple Inc. announced record earnings.",
            "A company announced record earnings.",
        ),
        ("You have 3 new messages.", "You have no new messages."),
        (
            "The meeting is at 9:30 in the morning.",
            "The meeting is in the morning.",
        ),
        ("Neither option is free.", "Both options are free."),
        (
            "The new plan costs less than $50 per month.",
            "The new plan costs more than $50 per month.",
        ),
        ("We cannot refund this item.", "We can refund this item."),
        ("The clinic is on 5 Main Street.", "The clinic is downtown."),
        ("Your balance is $0.00.", "Your balance is zero."),
        ("The offer ends in 2 days.", "The offer ends soon."),
        (
            "Bank of America raised its rate by 0.5%.",
            "The bank raised its rate.",
        ),
    ]
}

#[cfg(test)]
mod tests;
