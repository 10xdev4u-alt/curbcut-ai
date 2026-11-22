//! Extractive summarization with citations.
//!
//! [`summarize`] chunks a document into sentences, scores each candidate by the
//! mean term frequency of its words, skips a sentence that says nearly the same
//! thing as one already chosen, and emits the survivors **in reading order**,
//! each with the byte range and chunk id it came from. See
//! [`../../../docs/SUMMARIZE.md`](../../../docs/SUMMARIZE.md).
//!
//! Four rules hold:
//!
//! - **A key point cites its source.** [`KeyPoint::start`] and [`KeyPoint::end`]
//!   are the half-open byte range in the original document and
//!   [`KeyPoint::chunk_id`] is the chunk the sentence splitter produced, so a
//!   point can always be jumped back to.
//! - **Reading order is restored.** Selection is by score, but the emitted
//!   points are sorted by position, so the summary never reorders the document.
//!   [`Summary::reading_order_preserved`] checks it.
//! - **A near-duplicate is not a key point.** A candidate whose word overlap
//!   with an already-selected point exceeds [`MAX_REDUNDANCY`] is skipped.
//! - **The metric is reported, not gated.** [`rouge_1_recall`] and
//!   [`rouge_l_recall`] are computed against the source and stored in
//!   [`Summary::metrics`]; nothing refuses to produce a summary because of
//!   them.
//!
//! This module is **extractive**. An abstractive step is a model call and is
//! deliberately absent: the summary is a selection of the document's own
//! sentences, so it cannot invent a fact that is not there.
//!
//! The module keeps only the selected sentences and the source's length. It
//! does not retain the whole document, and it never logs a sentence.

use std::collections::HashMap;
use std::fmt;

use curbcut_core::id::ChunkId;

use crate::text_chunking::{chunk_text, ChunkKind, TextChunk};

/// The default number of key points.
pub const DEFAULT_MAX_POINTS: usize = 5;
/// The default fewest words a sentence may have to be a candidate.
pub const DEFAULT_MIN_WORDS: usize = 4;
/// The default most words a sentence may have to be a candidate.
pub const DEFAULT_MAX_WORDS: usize = 60;
/// The word overlap above which a candidate is a near-duplicate.
pub const MAX_REDUNDANCY: f32 = 0.8;

/// How many key points to extract, and which sentences may be candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryOptions {
    /// The most key points to emit.
    pub max_points: usize,
    /// The fewest words a sentence may have to be a candidate.
    pub min_words: usize,
    /// The most words a sentence may have to be a candidate.
    pub max_words: usize,
}

impl Default for SummaryOptions {
    fn default() -> Self {
        Self {
            max_points: DEFAULT_MAX_POINTS,
            min_words: DEFAULT_MIN_WORDS,
            max_words: DEFAULT_MAX_WORDS,
        }
    }
}

impl SummaryOptions {
    /// Creates options.
    pub const fn new(max_points: usize, min_words: usize, max_words: usize) -> Self {
        Self {
            max_points,
            min_words,
            max_words,
        }
    }
}

/// One extracted key point, cited to its source.
#[derive(Clone, PartialEq, Eq)]
pub struct KeyPoint {
    text: String,
    start: usize,
    end: usize,
    chunk_id: ChunkId,
    rank: usize,
}

impl KeyPoint {
    /// Creates a key point.
    pub fn new(text: String, start: usize, end: usize, chunk_id: ChunkId, rank: usize) -> Self {
        Self {
            text,
            start,
            end,
            chunk_id,
            rank,
        }
    }

    /// The sentence text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The byte offset where the sentence starts in the source.
    pub fn start(&self) -> usize {
        self.start
    }

    /// The byte offset where the sentence ends in the source.
    pub fn end(&self) -> usize {
        self.end
    }

    /// The id of the chunk the sentence came from.
    pub fn chunk_id(&self) -> ChunkId {
        self.chunk_id
    }

    /// The 1-based selection rank, by score. Not monotonic in reading order.
    pub fn rank(&self) -> usize {
        self.rank
    }

    /// The sentence's length in bytes.
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Whether the point is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// The slice of `source` this point cites, or `None` when the range does not
    /// fit.
    pub fn cited<'a>(&self, source: &'a str) -> Option<&'a str> {
        source.get(self.start..self.end)
    }

    /// A human-readable citation.
    pub fn citation(&self) -> String {
        format!(
            "chunk {}, bytes {}..{}",
            self.chunk_id.get(),
            self.start,
            self.end
        )
    }
}

impl fmt::Debug for KeyPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyPoint")
            .field("text_len", &self.text.len())
            .field("start", &self.start)
            .field("end", &self.end)
            .field("chunk_id", &self.chunk_id)
            .field("rank", &self.rank)
            .finish()
    }
}

/// What the summarizer measured. Reported, never a gate.
#[derive(Debug, Clone, PartialEq)]
pub struct SummaryMetrics {
    /// Sentences in the source, before filtering.
    pub source_sentences: usize,
    /// Sentences that passed the word-count filter.
    pub candidates: usize,
    /// Key points emitted.
    pub selected: usize,
    /// Words in the source's sentence chunks.
    pub source_words: usize,
    /// Words in the summary.
    pub summary_words: usize,
    /// ROUGE-1 recall against the source.
    pub rouge_1_recall: f32,
    /// ROUGE-L recall against the source.
    pub rouge_l_recall: f32,
}

/// An extractive summary.
#[derive(Clone, PartialEq)]
pub struct Summary {
    points: Vec<KeyPoint>,
    source_len_bytes: usize,
    options: SummaryOptions,
    metrics: SummaryMetrics,
}

impl Summary {
    /// Creates a summary from points and the source's length.
    pub fn new(
        points: Vec<KeyPoint>,
        source_len_bytes: usize,
        options: SummaryOptions,
        metrics: SummaryMetrics,
    ) -> Self {
        Self {
            points,
            source_len_bytes,
            options,
            metrics,
        }
    }

    /// The key points, in reading order.
    pub fn points(&self) -> &[KeyPoint] {
        &self.points
    }

    /// How many key points there are.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Whether the summary is empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// The source's length in bytes.
    pub fn source_len_bytes(&self) -> usize {
        self.source_len_bytes
    }

    /// The options used.
    pub const fn options(&self) -> &SummaryOptions {
        &self.options
    }

    /// The measured metrics.
    pub const fn metrics(&self) -> &SummaryMetrics {
        &self.metrics
    }

    /// The summary text: the key points joined by a space.
    pub fn text(&self) -> String {
        self.points
            .iter()
            .map(|point| point.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The summary's length as a fraction of the source, in `0.0..=1.0`.
    ///
    /// An empty source has a ratio of `0.0`.
    pub fn compression_ratio(&self) -> f32 {
        if self.source_len_bytes == 0 {
            return 0.0;
        }
        (self.text().len() as f32 / self.source_len_bytes as f32).min(1.0)
    }

    /// Whether the points are in ascending source order with no overlap.
    pub fn reading_order_preserved(&self) -> bool {
        self.points
            .windows(2)
            .all(|pair| pair[0].start <= pair[1].start && pair[0].end <= pair[1].end)
    }

    /// Every citation, in reading order.
    pub fn citations(&self) -> Vec<String> {
        self.points.iter().map(KeyPoint::citation).collect()
    }

    /// The slice of `source` each point cites, skipping a point whose range does
    /// not fit.
    pub fn cited_sentences<'a>(&self, source: &'a str) -> Vec<&'a str> {
        self.points
            .iter()
            .filter_map(|point| point.cited(source))
            .collect()
    }
}

impl fmt::Debug for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Summary")
            .field("point_count", &self.points.len())
            .field("source_len_bytes", &self.source_len_bytes)
            .field("options", &self.options)
            .field("metrics", &self.metrics)
            .finish()
    }
}

/// Summarizes `source` with `options`.
///
/// A source with no candidate sentence yields an empty summary. `options` may
/// be [`SummaryOptions::default`].
pub fn summarize(source: &str, options: &SummaryOptions) -> Summary {
    let chunks = chunk_text(source);
    let sentences: Vec<&TextChunk> = chunks
        .iter()
        .filter(|chunk| chunk.kind() == ChunkKind::Sentence)
        .collect();

    let mut term_frequency: HashMap<String, u32> = HashMap::new();
    let mut sentence_tokens: Vec<Vec<String>> = Vec::with_capacity(sentences.len());
    for chunk in &sentences {
        let tokens = tokenize(chunk.text(source));
        for token in &tokens {
            *term_frequency.entry(token.clone()).or_insert(0) += 1;
        }
        sentence_tokens.push(tokens);
    }

    let mut ranked: Vec<Candidate> = sentences
        .iter()
        .zip(sentence_tokens.iter())
        .filter_map(|(chunk, tokens)| {
            let score = mean_term_frequency(tokens, &term_frequency)?;
            Some(Candidate {
                chunk,
                tokens,
                score,
            })
        })
        .filter(|candidate| {
            let words = candidate.tokens.len();
            words >= options.min_words && words <= options.max_words
        })
        .collect();
    let candidates = ranked.len();
    ranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.chunk.start().cmp(&right.chunk.start()))
    });

    let mut selected: Vec<KeyPoint> = Vec::new();
    for candidate in ranked {
        if selected.len() == options.max_points {
            break;
        }
        let redundant = selected.iter().any(|point| {
            let words = tokenize(point.text());
            jaccard(candidate.tokens, &words) > MAX_REDUNDANCY
        });
        if redundant {
            continue;
        }
        selected.push(KeyPoint::new(
            candidate.chunk.text(source).to_string(),
            candidate.chunk.start(),
            candidate.chunk.end(),
            candidate.chunk.id(),
            selected.len() + 1,
        ));
    }
    selected.sort_by_key(|point| point.start);

    let summary_words = selected
        .iter()
        .flat_map(|point| tokenize(&point.text))
        .count();
    let summary_text = selected
        .iter()
        .map(|point| point.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let source_words = sentence_tokens.iter().map(Vec::len).sum();

    let metrics = SummaryMetrics {
        source_sentences: sentences.len(),
        candidates,
        selected: selected.len(),
        source_words,
        summary_words,
        rouge_1_recall: rouge_1_recall(&summary_text, source),
        rouge_l_recall: rouge_l_recall(&summary_text, source),
    };

    Summary::new(selected, source.len(), options.clone(), metrics)
}

/// A sentence that passed the word filter, with its score.
struct Candidate<'a> {
    chunk: &'a TextChunk,
    tokens: &'a [String],
    score: f32,
}

/// The mean term frequency of a sentence's words.
///
/// `None` when the sentence has no words, so an empty sentence is not a
/// candidate rather than a zero-score one.
fn mean_term_frequency(tokens: &[String], term_frequency: &HashMap<String, u32>) -> Option<f32> {
    if tokens.is_empty() {
        return None;
    }
    let total: u32 = tokens
        .iter()
        .map(|token| term_frequency.get(token).copied().unwrap_or(0))
        .sum();
    Some(total as f32 / tokens.len() as f32)
}

/// Lower-cased alphanumeric words, in order.
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.to_lowercase())
        .collect()
}

/// The Jaccard overlap of two token sequences: intersection over union.
fn jaccard(left: &[String], right: &[String]) -> f32 {
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }
    let left: std::collections::HashSet<&str> = left.iter().map(String::as_str).collect();
    let right: std::collections::HashSet<&str> = right.iter().map(String::as_str).collect();
    let union = left.union(&right).count();
    if union == 0 {
        return 0.0;
    }
    left.intersection(&right).count() as f32 / union as f32
}

/// ROUGE-1 recall: clipped unigram overlap over the reference's unigram count.
///
/// `0.0` when the reference has no words. Symmetric inputs are not required; the
/// first argument is the candidate and the second the reference.
pub fn rouge_1_recall(candidate: &str, reference: &str) -> f32 {
    let candidate = counts(candidate);
    let reference = counts(reference);
    let reference_total: u32 = reference.values().sum();
    if reference_total == 0 {
        return 0.0;
    }
    let overlap: u32 = candidate
        .iter()
        .map(|(word, count)| reference.get(word).copied().unwrap_or(0).min(*count))
        .sum();
    overlap as f32 / reference_total as f32
}

/// ROUGE-L recall: the longest common subsequence over the reference's word
/// count.
///
/// `0.0` when the reference has no words. The dynamic program keeps two rows,
/// so the cost is `O(candidate * reference)` time and `O(reference)` memory.
pub fn rouge_l_recall(candidate: &str, reference: &str) -> f32 {
    let candidate = tokenize(candidate);
    let reference = tokenize(reference);
    if reference.is_empty() {
        return 0.0;
    }
    let mut previous = vec![0_u32; reference.len() + 1];
    let mut current = vec![0_u32; reference.len() + 1];
    for left in &candidate {
        current.fill(0);
        for (column, right) in reference.iter().enumerate() {
            current[column + 1] = if left == right {
                previous[column] + 1
            } else {
                current[column].max(previous[column + 1])
            };
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[reference.len()] as f32 / reference.len() as f32
}

/// Clipped unigram counts.
fn counts(text: &str) -> HashMap<String, u32> {
    let mut counts: HashMap<String, u32> = HashMap::new();
    for word in tokenize(text) {
        *counts.entry(word).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests;
