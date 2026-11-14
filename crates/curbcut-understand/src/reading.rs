//! Reading-level comparison.
//!
//! Flesch reading ease is computed from word, sentence, and syllable counts.
//! The score maps onto [`ReadingLevel`], and a verdict compares the measured
//! band with the profile target. The functions are pure and never store text.

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
    /// Whether the measured band is at or below the target.
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
pub fn compare(text: &str, target: ReadingLevel) -> ReadingVerdict {
    let score = flesch_reading_ease(text);
    let level = ReadingLevel::from_flesch(score);
    ReadingVerdict {
        level,
        score,
        target,
        meets_target: level <= target,
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
}
