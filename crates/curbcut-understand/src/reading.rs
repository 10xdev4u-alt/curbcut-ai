//! Reading-level comparison.
//!
//! Flesch reading ease is computed from word, sentence, and syllable counts.
//! The score maps onto [`ReadingLevel`], and a verdict compares the measured
//! band with the profile target. The functions are pure and never store text.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use curbcut_core::types::ReadingLevel;

/// A reading-level verdict.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReadingVerdict {
    /// The measured band.
    pub level: ReadingLevel,
    /// The Flesch reading-ease score; higher is easier.
    pub score: f32,
    /// The profile target.
    pub target: ReadingLevel,
    /// Whether the text is measurable and its band is at or below the target.
    ///
    /// Unsupported scripts are never a passing verdict, even for a Complex
    /// target.
    pub meets_target: bool,
}

/// Counts syllables with a vowel-group heuristic.
///
/// This is an estimate, not a dictionary lookup. It is deterministic and good
/// enough to compare two texts; it is not a clinical reading-age measure.
pub fn count_syllables(word: &str) -> u32 {
    let word = word.trim().to_lowercase();
    if word.is_empty() {
        return 0;
    }
    let mut count = 0_u32;
    let mut previous_vowel = false;
    for ch in word.chars() {
        let vowel = matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u' | 'y');
        if vowel && !previous_vowel {
            count += 1;
        }
        previous_vowel = vowel;
    }
    // A trailing silent e does not add a syllable.
    if word.ends_with('e') && count > 1 {
        count -= 1;
    }
    count.max(1)
}

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .collect()
}

fn sentences(text: &str) -> usize {
    text.split(['.', '!', '?'])
        .filter(|part| !part.trim().is_empty())
        .count()
        .max(1)
}

/// Flesch reading ease. Higher is easier; 0 means no measurable text.
///
/// A text with no words, or only punctuation, scores 0. [`reading_level`] then
/// reports [`ReadingLevel::Complex`], the hardest band: an unmeasurable text
/// cannot be shown to meet an easy target.
pub fn flesch_reading_ease(text: &str) -> f32 {
    let words = words(text);
    if words.is_empty() {
        return 0.0;
    }
    let word_count = words.len() as f32;
    let sentence_count = sentences(text) as f32;
    let syllables: u32 = words.iter().map(|word| count_syllables(word)).sum();
    206.835 - 1.015 * (word_count / sentence_count) - 84.6 * (syllables as f32 / word_count)
}

/// The reading band for a text.
pub fn reading_level(text: &str) -> ReadingLevel {
    ReadingLevel::from_flesch(flesch_reading_ease(text))
}

/// Compares a text with a target band.
///
/// A text the scorer cannot measure (empty, non-Latin, or mixed script) gets a
/// zero score, the hardest band, and a negative verdict. Recomputing
/// `level <= target` here would wrongly pass a Complex target.
pub fn compare(text: &str, target: ReadingLevel) -> ReadingVerdict {
    let Some(score) = flesch_reading_ease_checked(text) else {
        return ReadingVerdict {
            level: ReadingLevel::Complex,
            score: 0.0,
            target,
            meets_target: false,
        };
    };
    let level = ReadingLevel::from_flesch(score);
    ReadingVerdict {
        level,
        score,
        target,
        meets_target: level <= target,
    }
}

/// Whether a text is written in a script the reading scores support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Script {
    /// No letters at all.
    Empty,
    /// Every letter is Latin.
    Latin,
    /// Some letters are Latin and some are not.
    Mixed,
    /// No Latin letters.
    NonLatin,
}

impl Script {
    /// Whether the reading scores are meaningful for this script.
    ///
    /// Only Latin is supported. A non-Latin or mixed text is flagged rather than
    /// mis-scored.
    pub fn is_supported(self) -> bool {
        matches!(self, Self::Latin)
    }
}

/// Classifies the script of a text by its letters.
pub fn classify_script(text: &str) -> Script {
    let mut latin = 0_usize;
    let mut other = 0_usize;
    for ch in text.chars() {
        if ch.is_alphabetic() {
            if is_latin(ch) {
                latin += 1;
            } else {
                other += 1;
            }
        }
    }
    match (latin, other) {
        (0, 0) => Script::Empty,
        (_, 0) => Script::Latin,
        (0, _) => Script::NonLatin,
        (_, _) => Script::Mixed,
    }
}

/// Whether a character is in a Latin range the scorer handles.
fn is_latin(ch: char) -> bool {
    matches!(ch as u32, 0x41..=0x5A | 0x61..=0x7A | 0xC0..=0x24F | 0x1E00..=0x1EFF)
}

/// Total syllables across words.
fn total_syllables(words: &[&str]) -> u32 {
    words.iter().map(|word| count_syllables(word)).sum()
}

/// Flesch reading ease, or `None` when the script is not supported.
pub fn flesch_reading_ease_checked(text: &str) -> Option<f32> {
    if !classify_script(text).is_supported() {
        return None;
    }
    Some(flesch_reading_ease(text))
}

/// Flesch-Kincaid grade level, or `None` when the script is not supported.
pub fn flesch_kincaid_grade(text: &str) -> Option<f32> {
    if !classify_script(text).is_supported() {
        return None;
    }
    let words = words(text);
    if words.is_empty() {
        return None;
    }
    let word_count = words.len() as f32;
    let sentence_count = sentences(text).max(1) as f32;
    let syllables = total_syllables(&words) as f32;
    Some(0.39 * (word_count / sentence_count) + 11.8 * (syllables / word_count) - 15.59)
}

/// SMOG grade, or `None` when the script is not supported or the text has fewer
/// than three sentences.
pub fn smog(text: &str) -> Option<f32> {
    if !classify_script(text).is_supported() {
        return None;
    }
    let words = words(text);
    if words.is_empty() {
        return None;
    }
    let sentence_count = sentences(text);
    if sentence_count < 3 {
        return None;
    }
    let polysyllables = words
        .iter()
        .filter(|word| count_syllables(word) >= 3)
        .count() as f32;
    Some(1.043 * (polysyllables * (30.0 / sentence_count as f32)).sqrt() + 3.1291)
}

/// Automated Reading Index, or `None` when the script is not supported.
pub fn ari(text: &str) -> Option<f32> {
    if !classify_script(text).is_supported() {
        return None;
    }
    let words = words(text);
    if words.is_empty() {
        return None;
    }
    let word_count = words.len() as f32;
    let sentence_count = sentences(text).max(1) as f32;
    let characters = words
        .iter()
        .map(|word| word.chars().filter(|ch| ch.is_alphanumeric()).count())
        .sum::<usize>() as f32;
    Some(4.71 * (characters / word_count) + 0.5 * (word_count / sentence_count) - 21.43)
}

/// A histogram of word lengths by syllable count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WordBands {
    /// One or two syllables.
    pub short: u32,
    /// Three syllables.
    pub medium: u32,
    /// Four or more syllables.
    pub long: u32,
}

impl WordBands {
    /// The total number of words banded.
    pub fn total(&self) -> u32 {
        self.short + self.medium + self.long
    }
}

/// A full reading score for one text.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadingScore {
    /// The script classification.
    pub script: Script,
    /// Flesch reading ease, or `None` when unsupported.
    pub flesch_ease: Option<f32>,
    /// Flesch-Kincaid grade level, or `None` when unsupported.
    pub flesch_kincaid_grade: Option<f32>,
    /// SMOG grade, or `None` when unsupported or too short.
    pub smog: Option<f32>,
    /// Automated Reading Index, or `None` when unsupported.
    pub ari: Option<f32>,
    /// Word count.
    pub words: u32,
    /// Sentence count.
    pub sentences: u32,
    /// Syllable count.
    pub syllables: u32,
    /// Word-length bands.
    pub word_bands: WordBands,
}

impl ReadingScore {
    /// Scores a text.
    pub fn compute(text: &str) -> Self {
        let script = classify_script(text);
        let supported = script.is_supported();
        let words = words(text);
        let syllable_counts: Vec<u32> = words.iter().map(|word| count_syllables(word)).collect();
        let syllables: u32 = syllable_counts.iter().sum();
        let word_count = words.len() as f32;
        let sentence_total = sentences(text).max(1);
        let sentence_count = sentence_total as f32;
        let word_bands = WordBands {
            short: syllable_counts.iter().filter(|&&n| n <= 2).count() as u32,
            medium: syllable_counts.iter().filter(|&&n| n == 3).count() as u32,
            long: syllable_counts.iter().filter(|&&n| n >= 4).count() as u32,
        };
        let (flesch_ease, flesch_kincaid_grade, smog, ari) = if !supported || words.is_empty() {
            if supported {
                (Some(0.0), None, None, None)
            } else {
                (None, None, None, None)
            }
        } else {
            let s = syllables as f32;
            let ease = 206.835 - 1.015 * (word_count / sentence_count) - 84.6 * (s / word_count);
            let grade = 0.39 * (word_count / sentence_count) + 11.8 * (s / word_count) - 15.59;
            let smog_grade = if sentence_count >= 3.0 {
                let polysyllables = syllable_counts.iter().filter(|&&n| n >= 3).count() as f32;
                Some(1.043 * (polysyllables * (30.0 / sentence_count)).sqrt() + 3.1291)
            } else {
                None
            };
            let characters = words
                .iter()
                .map(|word| word.chars().filter(|ch| ch.is_alphanumeric()).count())
                .sum::<usize>() as f32;
            let ari_index = Some(
                4.71 * (characters / word_count) + 0.5 * (word_count / sentence_count) - 21.43,
            );
            (Some(ease), Some(grade), smog_grade, ari_index)
        };
        Self {
            script,
            flesch_ease,
            flesch_kincaid_grade,
            smog,
            ari,
            words: words.len() as u32,
            sentences: sentence_total as u32,
            syllables,
            word_bands,
        }
    }

    /// Whether the scores are meaningful for this text.
    pub fn is_supported(&self) -> bool {
        self.script.is_supported()
    }
}

/// A non-cryptographic content hash for cache keys.
///
/// This is an identity key for the cache, not a security boundary.
pub fn content_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// A bounded cache of reading scores keyed by content hash.
#[derive(Debug, Clone)]
pub struct ScoreCache {
    entries: Vec<(u64, ReadingScore)>,
    limit: usize,
}

impl Default for ScoreCache {
    fn default() -> Self {
        Self::new(64)
    }
}

impl ScoreCache {
    /// Creates a cache holding at most `limit` entries.
    pub fn new(limit: usize) -> Self {
        Self {
            entries: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// The maximum number of entries.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// How many entries are cached.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Looks a score up by content hash.
    pub fn get(&self, hash: u64) -> Option<&ReadingScore> {
        self.entries
            .iter()
            .find(|(key, _)| *key == hash)
            .map(|(_, score)| score)
    }

    /// Caches a score, evicting the oldest entry when full.
    pub fn insert(&mut self, hash: u64, score: ReadingScore) {
        if let Some(entry) = self.entries.iter_mut().find(|(key, _)| *key == hash) {
            entry.1 = score;
            return;
        }
        if self.entries.len() >= self.limit {
            self.entries.remove(0);
        }
        self.entries.push((hash, score));
    }

    /// Returns the cached score for `text`, computing and caching it on a miss.
    pub fn score(&mut self, text: &str) -> ReadingScore {
        let hash = content_hash(text);
        if let Some(score) = self.get(hash) {
            return score.clone();
        }
        let score = ReadingScore::compute(text);
        self.insert(hash, score.clone());
        score
    }

    /// Drops every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllable_counting_is_deterministic() {
        assert_eq!(count_syllables("cat"), 1);
        assert_eq!(count_syllables("banana"), 3);
        assert_eq!(count_syllables("the"), 1);
        assert_eq!(count_syllables("silent"), 2);
        assert_eq!(count_syllables(""), 0);
    }

    #[test]
    fn an_empty_text_scores_zero() {
        assert_eq!(flesch_reading_ease(""), 0.0);
        assert_eq!(reading_level(""), ReadingLevel::Complex);
    }

    #[test]
    fn a_simple_sentence_is_easy() {
        let text = "The cat sat on the mat.";
        let verdict = compare(text, ReadingLevel::Plain);
        assert_eq!(verdict.level, ReadingLevel::Easy);
        assert!(verdict.score > 70.0, "score was {}", verdict.score);
        assert!(verdict.meets_target);
    }

    #[test]
    fn a_complex_sentence_is_harder_than_a_simple_one() {
        let simple = "The cat sat on the mat.";
        let complex = "The implementation necessitates a comprehensive reconceptualization.";
        assert!(
            flesch_reading_ease(complex) < flesch_reading_ease(simple),
            "complex must score lower"
        );
    }

    #[test]
    fn a_target_is_met_when_the_text_is_at_or_below_it() {
        let easy = "The cat sat on the mat.";
        let hard = "The implementation necessitates a comprehensive reconceptualization.";
        assert_eq!(reading_level(easy), ReadingLevel::Easy);
        assert_eq!(reading_level(hard), ReadingLevel::Complex);

        assert!(compare(easy, ReadingLevel::Easy).meets_target);
        assert!(compare(easy, ReadingLevel::Complex).meets_target);
        assert!(!compare(hard, ReadingLevel::Plain).meets_target);
        assert!(compare(hard, ReadingLevel::Complex).meets_target);
    }

    /// Unmeasurable text must never be reported as meeting a target. Every
    /// target is checked so a Complex target cannot hide the false green.
    #[test]
    fn unmeasurable_text_never_meets_any_target() {
        for text in [
            "Кот сидит на коврике и смотрит в окно.", // NonLatin
            "猫坐在窗边看外面。",                     // CJK
            "The cat 猫 sat on the mat.",             // Mixed
            "",
            "... !!! ???",
            "12345 !!!",
        ] {
            for target in [
                ReadingLevel::Easy,
                ReadingLevel::Plain,
                ReadingLevel::Complex,
            ] {
                let verdict = compare(text, target);
                assert!(
                    !verdict.meets_target,
                    "text {text:?}, target {target:?} must not pass"
                );
                assert_eq!(verdict.score, 0.0, "text {text:?}, target {target:?}");
                assert_eq!(
                    verdict.level,
                    ReadingLevel::Complex,
                    "text {text:?}, target {target:?}"
                );
                assert_eq!(verdict.target, target, "text {text:?}");
            }
        }
    }

    #[test]
    fn the_score_is_pure() {
        let text = "The cat sat on the mat.";
        assert_eq!(flesch_reading_ease(text), flesch_reading_ease(text));
    }

    #[test]
    fn punctuation_only_text_is_unmeasurable() {
        assert_eq!(flesch_reading_ease("... !!! ???"), 0.0);
        assert_eq!(reading_level("... !!! ???"), ReadingLevel::Complex);
    }

    #[test]
    fn unicode_text_is_measured_by_chars() {
        let text = "élan vital partout.";
        let score = flesch_reading_ease(text);
        assert!(score.is_finite(), "score was {score}");
        assert!(score > 0.0, "score was {score}");
    }

    #[test]
    fn a_consonant_only_word_has_one_syllable() {
        // `y` is treated as a vowel, so both words form a single group.
        assert_eq!(count_syllables("hmm"), 1);
        assert_eq!(count_syllables("rhythm"), 1);
    }

    #[test]
    fn latin_text_is_supported() {
        assert_eq!(classify_script("The cat sat on the mat."), Script::Latin);
        assert_eq!(classify_script("élan vital"), Script::Latin);
        assert!(classify_script("The cat sat on the mat.").is_supported());
    }

    #[test]
    fn non_latin_text_is_flagged() {
        assert_eq!(classify_script("Кот сидит."), Script::NonLatin);
        assert_eq!(classify_script("猫"), Script::NonLatin);
        assert!(!classify_script("Кот сидит.").is_supported());
    }

    #[test]
    fn mixed_text_is_flagged() {
        assert_eq!(classify_script("The cat Кот."), Script::Mixed);
        assert!(!classify_script("The cat Кот.").is_supported());
    }

    #[test]
    fn text_without_letters_is_empty() {
        assert_eq!(classify_script(""), Script::Empty);
        assert_eq!(classify_script("12345 !!!"), Script::Empty);
    }

    #[test]
    fn unsupported_scripts_are_not_scored() {
        let score = ReadingScore::compute("Кот сидит на коврике.");
        assert!(!score.is_supported());
        assert_eq!(score.script, Script::NonLatin);
        assert_eq!(score.flesch_ease, None);
        assert_eq!(score.flesch_kincaid_grade, None);
        assert_eq!(score.smog, None);
        assert_eq!(score.ari, None);
    }

    #[test]
    fn flesch_kincaid_matches_the_reference_formula() {
        // Three one-word sentences, three syllables each: 0.39 + 35.4 - 15.59.
        let grade = flesch_kincaid_grade("Banana. Banana. Banana.").expect("supported");
        assert!((grade - 20.2).abs() < 0.01, "grade was {grade}");
    }

    #[test]
    fn ari_matches_the_reference_formula() {
        // 18 characters over 3 words in 3 sentences: 28.26 + 0.5 - 21.43.
        let index = ari("Banana. Banana. Banana.").expect("supported");
        assert!((index - 7.33).abs() < 0.01, "ari was {index}");
    }

    #[test]
    fn smog_matches_the_reference_formula() {
        // Three polysyllables over three sentences: 1.043 * sqrt(30) + 3.1291.
        let grade = smog("Banana. Banana. Banana.").expect("three sentences");
        assert!((grade - 8.8418).abs() < 0.001, "smog was {grade}");
    }

    #[test]
    fn smog_is_none_below_three_sentences() {
        assert_eq!(smog("Banana. Banana."), None);
        assert_eq!(smog("Banana."), None);
        assert_eq!(smog(""), None);
    }

    #[test]
    fn word_bands_partition_the_words() {
        let score = ReadingScore::compute("The cat banana disappeared.");
        assert_eq!(score.words, 4);
        assert_eq!(score.word_bands.short, 2, "the, cat");
        assert_eq!(score.word_bands.medium, 1, "banana");
        assert_eq!(score.word_bands.long, 1, "disappeared");
        assert_eq!(score.word_bands.total(), 4);
    }

    #[test]
    fn a_supported_score_has_all_three_grades() {
        let score = ReadingScore::compute("Banana. Banana. Banana.");
        assert!(score.is_supported());
        assert!(score.flesch_ease.is_some());
        assert!(score.flesch_kincaid_grade.is_some());
        assert!(score.smog.is_some());
        assert!(score.ari.is_some());
        assert_eq!(score.sentences, 3);
        assert_eq!(score.syllables, 9);
    }

    #[test]
    fn the_score_cache_reuses_a_hash() {
        let text = "The cat sat on the mat.";
        let mut cache = ScoreCache::new(4);
        let first = cache.score(text);
        assert_eq!(cache.len(), 1);
        let second = cache.score(text);
        assert_eq!(first, second);
        assert_eq!(cache.len(), 1, "a hit does not grow the cache");
    }

    #[test]
    fn the_cache_evicts_the_oldest_entry() {
        let mut cache = ScoreCache::new(2);
        cache.score("one");
        cache.score("two");
        assert_eq!(cache.len(), 2);
        cache.score("three");
        assert_eq!(cache.len(), 2);
        assert!(cache.get(content_hash("one")).is_none(), "oldest evicted");
        assert!(cache.get(content_hash("three")).is_some());
    }

    #[test]
    fn the_cache_clears() {
        let mut cache = ScoreCache::new(4);
        cache.score("one");
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn the_documented_rules_match_the_code() {
        let doc = include_str!("../../../docs/READING_LEVEL.md");
        for word in ["SMOG", "ARI", "script", "cache"] {
            assert!(
                doc.contains(word),
                "docs/READING_LEVEL.md does not state `{word}`"
            );
        }
    }

    /// Builds the score from the reference helpers, independent of `compute`.
    fn expected_score(text: &str) -> ReadingScore {
        let words = words(text);
        ReadingScore {
            script: classify_script(text),
            flesch_ease: flesch_reading_ease_checked(text),
            flesch_kincaid_grade: flesch_kincaid_grade(text),
            smog: smog(text),
            ari: ari(text),
            words: words.len() as u32,
            sentences: sentences(text).max(1) as u32,
            syllables: total_syllables(&words),
            word_bands: WordBands {
                short: words
                    .iter()
                    .filter(|word| count_syllables(word) <= 2)
                    .count() as u32,
                medium: words
                    .iter()
                    .filter(|word| count_syllables(word) == 3)
                    .count() as u32,
                long: words
                    .iter()
                    .filter(|word| count_syllables(word) >= 4)
                    .count() as u32,
            },
        }
    }

    /// `PartialEq` on `f32` is not bit equality; check both.
    fn assert_same_score(actual: ReadingScore, expected: ReadingScore, text: &str) {
        assert_eq!(actual, expected, "text: {text:?}");
        assert_eq!(
            actual.flesch_ease.map(f32::to_bits),
            expected.flesch_ease.map(f32::to_bits),
            "flesch_ease bits, text: {text:?}"
        );
        assert_eq!(
            actual.flesch_kincaid_grade.map(f32::to_bits),
            expected.flesch_kincaid_grade.map(f32::to_bits),
            "flesch_kincaid_grade bits, text: {text:?}"
        );
        assert_eq!(
            actual.smog.map(f32::to_bits),
            expected.smog.map(f32::to_bits),
            "smog bits, text: {text:?}"
        );
        assert_eq!(
            actual.ari.map(f32::to_bits),
            expected.ari.map(f32::to_bits),
            "ari bits, text: {text:?}"
        );
    }

    #[test]
    fn compute_matches_the_reference_helpers_on_representative_texts() {
        let empty = "";
        let short_latin = "The cat sat on the mat.";
        let long_latin = "The curious little reader opened the window. Outside, a remarkable \
            procession of elephants meandered slowly past the garden. Nobody expected such a \
            visit, yet everybody watched in wonder! Could anyone explain where the enormous \
            animals came from?";
        let unsupported = "Кот сидит на коврике и смотрит в окно.";

        for text in [empty, short_latin, long_latin, unsupported] {
            assert_same_score(ReadingScore::compute(text), expected_score(text), text);
        }

        // Anchor the cases so the helpers cannot be wrong in the same way.
        let empty_score = ReadingScore::compute(empty);
        assert_eq!(empty_score.script, Script::Empty);
        assert_eq!(empty_score.words, 0);
        assert_eq!(empty_score.sentences, 1);
        assert_eq!(empty_score.flesch_ease, None);
        assert_eq!(empty_score.flesch_kincaid_grade, None);
        assert_eq!(empty_score.smog, None);
        assert_eq!(empty_score.ari, None);

        let short_score = ReadingScore::compute(short_latin);
        assert_eq!(short_score.script, Script::Latin);
        assert_eq!(short_score.words, 6);
        assert_eq!(short_score.sentences, 1);
        assert!(short_score.flesch_ease.is_some());
        assert_eq!(short_score.smog, None, "fewer than three sentences");

        let long_score = ReadingScore::compute(long_latin);
        assert_eq!(long_score.script, Script::Latin);
        assert_eq!(long_score.sentences, 4);
        assert!(long_score.smog.is_some());

        let unsupported_score = ReadingScore::compute(unsupported);
        assert_eq!(unsupported_score.script, Script::NonLatin);
        assert_eq!(unsupported_score.flesch_ease, None);
        assert_eq!(unsupported_score.flesch_kincaid_grade, None);
        assert_eq!(unsupported_score.smog, None);
        assert_eq!(unsupported_score.ari, None);

        // Mixed script and punctuation-only text exercise the other branches.
        for text in [
            "The cat Кот sat on the mat.",
            "... !!! ???",
            "12345 !!!",
            "élan vital partout.",
        ] {
            assert_same_score(ReadingScore::compute(text), expected_score(text), text);
        }
    }

    #[test]
    fn compute_matches_the_reference_helpers_across_lengths_and_scripts() {
        // Vary the sentence count across the SMOG <3 threshold and the word
        // count from zero upward, checking every score bit against the helpers.
        for sentence_count in 0..=12 {
            let text = "Banana. ".repeat(sentence_count);
            assert_same_score(ReadingScore::compute(&text), expected_score(&text), &text);
        }
        for text in [
            "Banana.",
            "Banana. Banana.",
            "hmm rhythm silent",
            "猫 猫 猫",
        ] {
            assert_same_score(ReadingScore::compute(text), expected_score(text), text);
        }
    }
}
