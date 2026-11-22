//! Tests for the extractive summarizer.

use super::*;

/// Asserts two `f32` values are equal within a tolerance.
fn near(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() < 1e-6
}

/// A four-sentence document where the third sentence repeats "the" most and so
/// scores highest, while the first sentence is last by score.
const FOUR: &str = "First the cache stores a hash. Then the cache reuses the hash for every read. The adapter paints the grid. Finally the engine sleeps.";

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/SUMMARIZE.md");
    for word in [
        "key point",
        "reading order",
        "ROUGE",
        "extractive",
        "citation",
    ] {
        assert!(
            doc.contains(word),
            "docs/SUMMARIZE.md does not state `{word}`"
        );
    }
}

#[test]
fn options_default_to_the_documented_constants() {
    let options = SummaryOptions::default();
    assert_eq!(options.max_points, DEFAULT_MAX_POINTS);
    assert_eq!(options.min_words, DEFAULT_MIN_WORDS);
    assert_eq!(options.max_words, DEFAULT_MAX_WORDS);
    assert_eq!(DEFAULT_MAX_POINTS, 5);
    assert_eq!(DEFAULT_MIN_WORDS, 4);
    assert_eq!(DEFAULT_MAX_WORDS, 60);
    assert_eq!(
        SummaryOptions::new(2, 1, 3),
        SummaryOptions {
            max_points: 2,
            min_words: 1,
            max_words: 3,
        }
    );
}

#[test]
fn every_sentence_is_a_candidate_and_the_summary_is_the_source() {
    let summary = summarize(FOUR, &SummaryOptions::default());
    assert_eq!(summary.len(), 4);
    assert_eq!(summary.metrics().source_sentences, 4);
    assert_eq!(summary.metrics().candidates, 4);
    assert_eq!(summary.metrics().selected, 4);
    // Every candidate is selected, so the joined points reconstruct the source.
    assert_eq!(summary.text(), FOUR);
    assert!(near(summary.metrics().rouge_1_recall, 1.0));
    assert!(near(summary.metrics().rouge_l_recall, 1.0));
}

#[test]
fn selection_is_by_score_but_output_is_in_reading_order() {
    let summary = summarize(FOUR, &SummaryOptions::new(2, 4, 60));
    let texts = summary
        .points()
        .iter()
        .map(|point| point.text())
        .collect::<Vec<_>>();
    // The third sentence scores highest, the second next; both are emitted in
    // the order they appear, not the order they ranked.
    assert_eq!(
        texts,
        vec![
            "Then the cache reuses the hash for every read.",
            "The adapter paints the grid.",
        ]
    );
    assert!(summary.reading_order_preserved());
    assert_eq!(summary.metrics().selected, 2);
}

#[test]
fn ranks_are_selection_order_and_not_monotonic_in_reading_order() {
    let summary = summarize(FOUR, &SummaryOptions::default());
    let ranks = summary
        .points()
        .iter()
        .map(|point| point.rank())
        .collect::<Vec<_>>();
    // Reading order is first, second, third, fourth; their ranks are 4, 2, 1, 3.
    assert_eq!(ranks, vec![4, 2, 1, 3]);
}

#[test]
fn each_point_cites_its_exact_source_slice() {
    let summary = summarize(FOUR, &SummaryOptions::default());
    assert_eq!(summary.citations().len(), 4);
    for point in summary.points() {
        assert_eq!(point.cited(FOUR), Some(point.text()));
        assert_eq!(&FOUR[point.start()..point.end()], point.text());
        assert_eq!(
            point.citation(),
            format!(
                "chunk {}, bytes {}..{}",
                point.chunk_id().get(),
                point.start(),
                point.end()
            )
        );
    }
    let cited = summary.cited_sentences(FOUR);
    assert_eq!(
        cited,
        summary
            .points()
            .iter()
            .map(KeyPoint::text)
            .collect::<Vec<_>>()
    );
}

#[test]
fn starts_are_ascending_with_no_overlap() {
    let summary = summarize(FOUR, &SummaryOptions::default());
    let starts = summary
        .points()
        .iter()
        .map(|point| point.start())
        .collect::<Vec<_>>();
    assert_eq!(starts, vec![0, 31, 78, 107]);
    for pair in summary.points().windows(2) {
        assert!(pair[0].start() <= pair[1].start());
        assert!(pair[0].end() <= pair[1].end());
    }
}

#[test]
fn a_near_duplicate_is_skipped() {
    let source = "The cache stores the hash for the read. The cache stores the hash for the read now. The adapter paints the grid.";
    let summary = summarize(source, &SummaryOptions::default());
    let texts = summary
        .points()
        .iter()
        .map(|point| point.text())
        .collect::<Vec<_>>();
    // The first of the pair scores higher; the second is a near-duplicate.
    assert_eq!(
        texts,
        vec![
            "The cache stores the hash for the read.",
            "The adapter paints the grid.",
        ]
    );
    assert_eq!(summary.metrics().candidates, 3);
    assert_eq!(summary.metrics().selected, 2);
}

#[test]
fn the_word_filters_exclude_short_and_long_sentences() {
    let source = "It rains. The cache stores the hash for the read.";
    let short = summarize(source, &SummaryOptions::new(5, 4, 60));
    assert_eq!(short.metrics().source_sentences, 2);
    assert_eq!(short.metrics().candidates, 1);
    assert_eq!(short.len(), 1);
    assert_eq!(
        short.points()[0].text(),
        "The cache stores the hash for the read."
    );

    let long = summarize(source, &SummaryOptions::new(5, 1, 3));
    assert_eq!(long.metrics().candidates, 1);
    assert_eq!(long.len(), 1);
    assert_eq!(long.points()[0].text(), "It rains.");
}

#[test]
fn zero_points_or_an_inverted_range_yields_an_empty_summary() {
    let none = summarize(FOUR, &SummaryOptions::new(0, 4, 60));
    assert!(none.is_empty());
    assert_eq!(none.metrics().selected, 0);
    assert_eq!(none.metrics().candidates, 4);

    let inverted = summarize(FOUR, &SummaryOptions::new(5, 10, 4));
    assert!(inverted.is_empty());
    assert_eq!(inverted.metrics().candidates, 0);
}

#[test]
fn an_empty_source_yields_an_empty_summary() {
    let summary = summarize("", &SummaryOptions::default());
    assert!(summary.is_empty());
    assert_eq!(summary.source_len_bytes(), 0);
    assert_eq!(summary.compression_ratio(), 0.0);
    assert_eq!(summary.metrics().source_sentences, 0);
    assert_eq!(summary.metrics().selected, 0);
    assert_eq!(summary.metrics().rouge_1_recall, 0.0);
    assert_eq!(summary.metrics().rouge_l_recall, 0.0);
    assert!(summary.citations().is_empty());
    assert!(summary.cited_sentences("").is_empty());
}

#[test]
fn a_source_with_no_sentence_chunk_yields_an_empty_summary() {
    let source = "# Title\n\n- item one\n- item two\n\n| a | b |\n| --- | --- |\n| c | d |";
    let summary = summarize(source, &SummaryOptions::default());
    assert!(summary.is_empty());
    assert_eq!(summary.metrics().source_sentences, 0);
    assert_eq!(summary.metrics().candidates, 0);
    assert!(near(summary.compression_ratio(), 0.0));
}

#[test]
fn the_summary_never_invents_text() {
    let summary = summarize(FOUR, &SummaryOptions::new(3, 4, 60));
    assert_eq!(summary.len(), 3);
    for point in summary.points() {
        assert!(FOUR.contains(point.text()));
        assert!(!point.text().is_empty());
        assert_eq!(point.len(), point.text().len());
        assert!(!point.is_empty());
    }
    assert_eq!(
        summary.text(),
        summary
            .points()
            .iter()
            .map(KeyPoint::text)
            .collect::<Vec<_>>()
            .join(" ")
    );
}

#[test]
fn compression_ratio_is_bounded_and_tracks_length() {
    let summary = summarize(FOUR, &SummaryOptions::new(1, 4, 60));
    let ratio = summary.compression_ratio();
    assert!(ratio > 0.0 && ratio <= 1.0);
    assert!(near(ratio, summary.text().len() as f32 / FOUR.len() as f32));

    let whole = summarize(FOUR, &SummaryOptions::default());
    assert!(near(whole.compression_ratio(), 1.0));
}

#[test]
fn rouge_1_recall_counts_clipped_unigrams() {
    assert!(near(rouge_1_recall("a b c", "a b c"), 1.0));
    assert!(near(rouge_1_recall("a b", "a b c"), 2.0 / 3.0));
    assert!(near(rouge_1_recall("a a b", "a b"), 1.0));
    assert!(near(rouge_1_recall("a b c", "a b"), 1.0));
    assert_eq!(rouge_1_recall("", "a b"), 0.0);
    assert_eq!(rouge_1_recall("a b", ""), 0.0);
    assert_eq!(rouge_1_recall("", ""), 0.0);
    // Case and punctuation are not word boundaries of their own.
    assert!(near(rouge_1_recall("A, b!", "a b"), 1.0));
}

#[test]
fn rouge_l_recall_is_the_longest_common_subsequence() {
    assert!(near(rouge_l_recall("a b c", "a b c"), 1.0));
    assert!(near(rouge_l_recall("a c", "a b c"), 2.0 / 3.0));
    assert!(near(rouge_l_recall("a b", "a b c"), 2.0 / 3.0));
    assert!(near(rouge_l_recall("c b a", "a b c"), 1.0 / 3.0));
    assert_eq!(rouge_l_recall("", "a b"), 0.0);
    assert_eq!(rouge_l_recall("a b", ""), 0.0);
    assert_eq!(rouge_l_recall("", ""), 0.0);
}

#[test]
fn metrics_report_counts_and_scores() {
    let summary = summarize(FOUR, &SummaryOptions::new(2, 4, 60));
    let metrics = summary.metrics();
    assert_eq!(metrics.source_sentences, 4);
    assert_eq!(metrics.candidates, 4);
    assert_eq!(metrics.selected, 2);
    assert_eq!(metrics.source_words, 24);
    assert!(metrics.summary_words > 0);
    assert!(metrics.rouge_1_recall > 0.0);
    assert!(metrics.rouge_l_recall > 0.0);
    assert_eq!(summary.options().max_points, 2);
    assert_eq!(summary.source_len_bytes(), FOUR.len());
}

#[test]
fn debug_prints_lengths_and_offsets_not_the_text() {
    let summary = summarize(FOUR, &SummaryOptions::new(1, 4, 60));
    let debug = format!("{summary:?}");
    assert!(!debug.contains("adapter"));
    assert!(!debug.contains("cache"));
    assert!(debug.contains("point_count"));
    assert!(debug.contains("source_len_bytes"));
    let point_debug = format!("{:?}", summary.points()[0]);
    assert!(!point_debug.contains("adapter"));
    assert!(point_debug.contains("text_len"));
}
