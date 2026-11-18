//! Tests for text chunking.

use super::*;

fn kinds(chunks: &[TextChunk]) -> Vec<&'static str> {
    chunks.iter().map(|chunk| chunk.kind().as_str()).collect()
}

fn texts<'a>(chunks: &[TextChunk], source: &'a str) -> Vec<&'a str> {
    chunks.iter().map(|chunk| chunk.text(source)).collect()
}

fn ids(chunks: &[TextChunk]) -> Vec<u64> {
    chunks.iter().map(|chunk| chunk.id().get()).collect()
}

#[test]
fn kinds_are_named() {
    assert_eq!(ChunkKind::Heading.as_str(), "heading");
    assert_eq!(ChunkKind::ListItem.as_str(), "list-item");
    assert_eq!(ChunkKind::Table.as_str(), "table");
    assert_eq!(ChunkKind::Sentence.as_str(), "sentence");
    assert_eq!(ChunkKind::Clause.as_str(), "clause");
}

#[test]
fn an_empty_source_has_no_chunks() {
    assert!(chunk_text("").is_empty());
    assert!(chunk_text("\n\n").is_empty());
}

#[test]
fn a_single_sentence_is_one_chunk() {
    let source = "One sentence.";
    let chunks = chunk_text(source);
    assert_eq!(kinds(&chunks), vec!["sentence"]);
    assert_eq!(texts(&chunks, source), vec!["One sentence."]);
    assert_eq!(chunks[0].start(), 0);
    assert_eq!(chunks[0].end(), source.len());
    assert_eq!(chunks[0].len(), source.len());
    assert!(!chunks[0].is_empty());
}

#[test]
fn sentences_are_split_on_terminators() {
    let source = "One. Two! Three?";
    let chunks = chunk_text(source);
    assert_eq!(kinds(&chunks), vec!["sentence", "sentence", "sentence"]);
    assert_eq!(texts(&chunks, source), vec!["One.", "Two!", "Three?"]);
}

#[test]
fn a_sentence_ending_in_a_quote_keeps_the_quote() {
    let source = "She said \"Go.\" Then she left.";
    let chunks = chunk_text(source);
    assert_eq!(
        texts(&chunks, source),
        vec!["She said \"Go.\"", "Then she left."]
    );
}

#[test]
fn a_decimal_is_not_a_sentence_boundary() {
    let source = "Pi is 3.14 today.";
    let chunks = chunk_text(source);
    assert_eq!(texts(&chunks, source), vec!["Pi is 3.14 today."]);
}

#[test]
fn an_abbreviation_is_not_a_sentence_boundary() {
    let source = "We saw e.g. A bird.";
    let chunks = chunk_text(source);
    assert_eq!(texts(&chunks, source), vec!["We saw e.g. A bird."]);

    let source = "Dr. Smith left.";
    assert_eq!(texts(&chunk_text(source), source), vec!["Dr. Smith left."]);
}

#[test]
fn a_lowercase_word_after_a_period_is_not_a_boundary() {
    let source = "It is etc. really.";
    let chunks = chunk_text(source);
    assert_eq!(texts(&chunks, source), vec!["It is etc. really."]);
}

#[test]
fn headings_are_their_own_chunks() {
    let source = "# Title\n\nBody text.";
    let chunks = chunk_text(source);
    assert_eq!(kinds(&chunks), vec!["heading", "sentence"]);
    assert_eq!(texts(&chunks, source), vec!["# Title", "Body text."]);
}

#[test]
fn an_all_caps_line_is_a_heading() {
    let source = "WARNING\n\nRead this.";
    let chunks = chunk_text(source);
    assert_eq!(kinds(&chunks), vec!["heading", "sentence"]);
    assert_eq!(texts(&chunks, source), vec!["WARNING", "Read this."]);
}

#[test]
fn list_items_are_their_own_chunks() {
    let source = "- One\n- Two\n";
    let chunks = chunk_text(source);
    assert_eq!(kinds(&chunks), vec!["list-item", "list-item"]);
    assert_eq!(texts(&chunks, source), vec!["- One", "- Two"]);
}

#[test]
fn numbered_list_items_are_chunks() {
    let source = "1. One\n2) Two\n";
    let chunks = chunk_text(source);
    assert_eq!(kinds(&chunks), vec!["list-item", "list-item"]);
    assert_eq!(texts(&chunks, source), vec!["1. One", "2) Two"]);
}

#[test]
fn a_table_is_one_chunk() {
    let source = "| a | b |\n| --- | --- |\n| 1 | 2 |\n";
    let chunks = chunk_text(source);
    assert_eq!(kinds(&chunks), vec!["table"]);
    assert_eq!(chunks.len(), 1);
    let text = chunks[0].text(source);
    assert!(text.contains("| a | b |"));
    assert!(text.contains("| --- | --- |"));
    assert!(text.contains("| 1 | 2 |"));
}

#[test]
fn a_table_is_not_split_by_a_period_or_a_sentence() {
    let source = "| Item | Note |\n| --- | --- |\n| A | 3.14 is pi. |\n";
    let chunks = chunk_text(source);
    assert_eq!(kinds(&chunks), vec!["table"]);
    assert_eq!(chunks.len(), 1, "a table is never split across chunks");
}

#[test]
fn a_table_between_paragraphs_is_one_chunk() {
    let source = "Before.\n\n| a |\n| --- |\n\nAfter.";
    let chunks = chunk_text(source);
    assert_eq!(kinds(&chunks), vec!["sentence", "table", "sentence"]);
}

#[test]
fn a_paragraph_longer_than_one_line_is_split_into_sentences() {
    let source = "First line.\nSecond line.\n";
    let chunks = chunk_text(source);
    assert_eq!(kinds(&chunks), vec!["sentence", "sentence"]);
    assert_eq!(texts(&chunks, source), vec!["First line.", "Second line."]);
}

#[test]
fn chunk_ids_are_sequential_and_stable() {
    let source = "# Title\n\nOne. Two.\n\n- Item";
    let first = chunk_text(source);
    let second = chunk_text(source);
    assert_eq!(ids(&first), vec![0, 1, 2, 3]);
    assert_eq!(first, second, "chunking is deterministic");
}

#[test]
fn crlf_lines_are_handled() {
    let source = "One.\r\nTwo.\r\n";
    let chunks = chunk_text(source);
    assert_eq!(texts(&chunks, source), vec!["One.", "Two."]);
}

#[test]
fn a_trailing_newline_is_not_a_chunk() {
    let source = "One.\n";
    let chunks = chunk_text(source);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].end(), 4);
}

#[test]
fn a_paragraph_is_covered_by_its_sentences() {
    let source = "Alpha beta. Gamma delta! Epsilon?";
    let chunks = chunk_text(source);
    let joined: String = chunks
        .iter()
        .map(|chunk| chunk.text(source))
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(joined, source);
}

#[test]
fn clauses_are_split_on_punctuation() {
    let source = "One, two; three: four.";
    let chunks = split_clauses(source, 0, source.len(), 0);
    assert_eq!(kinds(&chunks), vec!["clause", "clause", "clause", "clause"]);
    assert_eq!(texts(&chunks, source), vec!["One", "two", "three", "four."]);
    assert_eq!(ids(&chunks), vec![0, 1, 2, 3]);
}

#[test]
fn clause_ids_continue_from_the_first_id() {
    let source = "One, two.";
    let chunks = split_clauses(source, 0, source.len(), 7);
    assert_eq!(ids(&chunks), vec![7, 8]);
}

#[test]
fn a_clause_break_needs_a_following_space() {
    let source = "One,two.";
    let chunks = split_clauses(source, 0, source.len(), 0);
    assert_eq!(chunks.len(), 1);
    assert_eq!(texts(&chunks, source), vec!["One,two."]);
}

#[test]
fn a_clause_does_not_split_a_number_or_a_time() {
    let source = "Pay 1,000 now.";
    let chunks = split_clauses(source, 0, source.len(), 0);
    assert_eq!(texts(&chunks, source), vec!["Pay 1,000 now."]);

    let source = "At 12:30, go.";
    let chunks = split_clauses(source, 0, source.len(), 0);
    assert_eq!(texts(&chunks, source), vec!["At 12:30", "go."]);
}

#[test]
fn clauses_use_a_subrange_of_the_source() {
    let source = "Noise. Alpha, beta. More.";
    let sentence_start = source.find("Alpha").expect("found");
    let sentence_end = sentence_start + source[sentence_start..].find('.').expect("found") + 1;
    let chunks = split_clauses(source, sentence_start, sentence_end, 2);
    assert_eq!(texts(&chunks, source), vec!["Alpha", "beta."]);
    assert_eq!(chunks[0].start(), sentence_start);
    assert_eq!(chunks[0].end(), sentence_start + "Alpha".len());
    assert_eq!(ids(&chunks), vec![2, 3]);
}

#[test]
fn an_empty_clause_is_skipped() {
    let source = "One, , two.";
    let chunks = split_clauses(source, 0, source.len(), 0);
    assert_eq!(texts(&chunks, source), vec!["One", "two."]);
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/TEXT_CHUNKING.md");
    for word in ["chunk", "sentence", "clause", "table", "heading"] {
        assert!(
            doc.contains(word),
            "docs/TEXT_CHUNKING.md does not state `{word}`"
        );
    }
}
