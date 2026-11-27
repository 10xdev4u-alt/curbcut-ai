//! Chunk plain text into headings, list items, tables, sentences, and clauses.
//!
//! [`chunk_text`] walks the source once and returns half-open byte ranges, so a
//! chunk borrows the source instead of copying it. Three rules hold:
//!
//! - **The order is the reading order.** Chunks are emitted top to bottom with
//!   ids sequential from zero, so a chunk id is stable for highlighting.
//! - **A table stays whole.** Consecutive table lines become one
//!   [`ChunkKind::Table`] chunk and are never split across chunks.
//! - **No mid-word splits.** A period inside a number, a whole-word
//!   abbreviation, or an initialism (`U.S.`) does not end a sentence, and a
//!   clause break needs a following space. The initialism rule is conservative:
//!   a short capitalised word followed by a single letter and a period may be
//!   kept in one chunk rather than split.
//!
//! [`split_clauses`] cuts a sentence into clauses. The module reads text the
//! caller passes and never logs it. See
//! [`../../../docs/TEXT_CHUNKING.md`](../../../docs/TEXT_CHUNKING.md).

use curbcut_core::id::ChunkId;

/// Abbreviations whose final period does not end a sentence.
const ABBREVIATIONS: [&str; 12] = [
    "e.g.", "i.e.", "etc.", "mr.", "mrs.", "ms.", "dr.", "vs.", "fig.", "inc.", "ltd.", "prof.",
];

/// The kind of a text chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChunkKind {
    /// A heading line.
    Heading,
    /// One list item.
    ListItem,
    /// A whole table.
    Table,
    /// A sentence within a paragraph.
    Sentence,
    /// A clause within a sentence.
    Clause,
}

impl ChunkKind {
    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Heading => "heading",
            Self::ListItem => "list-item",
            Self::Table => "table",
            Self::Sentence => "sentence",
            Self::Clause => "clause",
        }
    }
}

/// A half-open byte range into the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextChunk {
    id: ChunkId,
    kind: ChunkKind,
    start: usize,
    end: usize,
}

impl TextChunk {
    /// Creates a chunk over `start..end`.
    pub const fn new(id: ChunkId, kind: ChunkKind, start: usize, end: usize) -> Self {
        Self {
            id,
            kind,
            start,
            end,
        }
    }

    /// The chunk id.
    pub const fn id(&self) -> ChunkId {
        self.id
    }

    /// The chunk kind.
    pub const fn kind(&self) -> ChunkKind {
        self.kind
    }

    /// The byte offset where the chunk starts.
    pub const fn start(&self) -> usize {
        self.start
    }

    /// The byte offset where the chunk ends.
    pub const fn end(&self) -> usize {
        self.end
    }

    /// The chunk's length in bytes.
    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    /// Whether the chunk is empty.
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// The chunk's text, borrowed from `source`.
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}

/// Chunks `source` into headings, list items, tables, and sentences.
pub fn chunk_text(source: &str) -> Vec<TextChunk> {
    let lines = lines_with_offsets(source);
    let mut chunks = Vec::new();
    let mut next_id = 0_u64;
    let mut index = 0_usize;
    while index < lines.len() {
        let (start, end, text) = lines[index];
        if text.is_empty() {
            index += 1;
        } else if is_table_line(text) {
            let block_start = start;
            let mut block_end = end;
            let mut cursor = index + 1;
            while cursor < lines.len() && is_table_line(lines[cursor].2) {
                block_end = lines[cursor].1;
                cursor += 1;
            }
            push_trimmed(
                &mut chunks,
                &mut next_id,
                ChunkKind::Table,
                source,
                block_start,
                block_end,
            );
            index = cursor;
        } else if is_list_item(text) {
            push_trimmed(
                &mut chunks,
                &mut next_id,
                ChunkKind::ListItem,
                source,
                start,
                end,
            );
            index += 1;
        } else if is_heading(text) {
            push_trimmed(
                &mut chunks,
                &mut next_id,
                ChunkKind::Heading,
                source,
                start,
                end,
            );
            index += 1;
        } else {
            let paragraph_start = start;
            let mut paragraph_end = end;
            let mut cursor = index + 1;
            while cursor < lines.len() {
                let (_, line_end, line) = lines[cursor];
                if line.is_empty() || is_special_line(line) {
                    break;
                }
                paragraph_end = line_end;
                cursor += 1;
            }
            push_sentences(
                &mut chunks,
                &mut next_id,
                source,
                paragraph_start,
                paragraph_end,
            );
            index = cursor;
        }
    }
    chunks
}

/// Splits the byte range `start..end` of `source` into clauses.
///
/// Ids continue from `first_id`. A clause break needs a following space, so a
/// comma inside `1,000` or a colon inside `12:30` does not split. Returns no
/// clauses when `start..end` is not a valid range on char boundaries.
pub fn split_clauses(source: &str, start: usize, end: usize, first_id: u64) -> Vec<TextChunk> {
    let Some(text) = source.get(start..end) else {
        return Vec::new();
    };
    let bytes = text.as_bytes();
    let mut chunks = Vec::new();
    let mut next_id = first_id;
    let mut clause_start = 0_usize;
    let mut index = 0_usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if matches!(byte, b',' | b';' | b':')
            && index + 1 < bytes.len()
            && bytes[index + 1].is_ascii_whitespace()
            && !is_numeric_break(bytes, index)
        {
            push_clause(&mut chunks, &mut next_id, text, clause_start, index, start);
            clause_start = index + 1;
        }
        index += 1;
    }
    push_clause(
        &mut chunks,
        &mut next_id,
        text,
        clause_start,
        bytes.len(),
        start,
    );
    chunks
}

/// Pushes the sentences found in `source[start..end]`.
fn push_sentences(
    chunks: &mut Vec<TextChunk>,
    next_id: &mut u64,
    source: &str,
    start: usize,
    end: usize,
) {
    for (sentence_start, sentence_end) in sentence_ranges(&source[start..end]) {
        push(
            chunks,
            next_id,
            ChunkKind::Sentence,
            start + sentence_start,
            start + sentence_end,
        );
    }
}

/// Appends a chunk, skipping an empty range.
fn push(chunks: &mut Vec<TextChunk>, next_id: &mut u64, kind: ChunkKind, start: usize, end: usize) {
    if start >= end {
        return;
    }
    chunks.push(TextChunk::new(ChunkId(*next_id), kind, start, end));
    *next_id += 1;
}

/// Appends a chunk over the trimmed range, skipping a blank range.
fn push_trimmed(
    chunks: &mut Vec<TextChunk>,
    next_id: &mut u64,
    kind: ChunkKind,
    source: &str,
    start: usize,
    end: usize,
) {
    if let Some((trimmed_start, trimmed_end)) = trimmed_range(source, start, end) {
        push(chunks, next_id, kind, trimmed_start, trimmed_end);
    }
}

/// Appends one clause, trimmed, skipping an empty clause.
fn push_clause(
    chunks: &mut Vec<TextChunk>,
    next_id: &mut u64,
    text: &str,
    start: usize,
    end: usize,
    base: usize,
) {
    if let Some((clause_start, clause_end)) = trimmed_range(text, start, end) {
        push(
            chunks,
            next_id,
            ChunkKind::Clause,
            base + clause_start,
            base + clause_end,
        );
    }
}

/// The trimmed byte range within `text[start..end]`, or `None` when blank.
fn trimmed_range(text: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut trimmed_start = start;
    let mut trimmed_end = end;
    while trimmed_start < trimmed_end && bytes[trimmed_start].is_ascii_whitespace() {
        trimmed_start += 1;
    }
    while trimmed_end > trimmed_start && bytes[trimmed_end - 1].is_ascii_whitespace() {
        trimmed_end -= 1;
    }
    if trimmed_start == trimmed_end {
        None
    } else {
        Some((trimmed_start, trimmed_end))
    }
}

/// One line's content range and trimmed text.
fn lines_with_offsets(source: &str) -> Vec<(usize, usize, &str)> {
    let mut lines = Vec::new();
    let mut start = 0_usize;
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            let mut end = index;
            if end > start && source.as_bytes()[end - 1] == b'\r' {
                end -= 1;
            }
            lines.push((start, end, source[start..end].trim()));
            start = index + 1;
        }
    }
    if start < source.len() {
        lines.push((start, source.len(), source[start..].trim()));
    }
    lines
}

/// Whether a line starts a table, a heading, or a list item.
fn is_special_line(text: &str) -> bool {
    is_table_line(text) || is_heading(text) || is_list_item(text)
}

/// Whether a line is part of a table.
fn is_table_line(text: &str) -> bool {
    text.starts_with('|')
}

/// Whether a line is a heading.
fn is_heading(text: &str) -> bool {
    let hashes = text.chars().take_while(|byte| *byte == '#').count();
    if (1..=6).contains(&hashes) && text[hashes..].starts_with(' ') {
        return true;
    }
    let mut letters = text.chars().filter(|byte| byte.is_alphabetic());
    match letters.next() {
        Some(first) => text.len() <= 60 && first.is_uppercase() && letters.all(char::is_uppercase),
        None => false,
    }
}

/// Whether a line is a list item.
fn is_list_item(text: &str) -> bool {
    for marker in ["- ", "* ", "+ ", "\u{2022} "] {
        if let Some(rest) = text.strip_prefix(marker) {
            return !rest.trim().is_empty();
        }
    }
    let digits = text.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return false;
    }
    let rest = &text[digits..];
    if rest.starts_with(". ") || rest.starts_with(") ") {
        return !rest[2..].trim().is_empty();
    }
    false
}

/// The sentence ranges within a paragraph.
fn sentence_ranges(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut ranges = Vec::new();
    let mut start = 0_usize;
    let mut index = 0_usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if matches!(byte, b'.' | b'!' | b'?') {
            let mut end = index + 1;
            while end < bytes.len() && matches!(bytes[end], b'"' | b'\'' | b')' | b']' | b'}') {
                end += 1;
            }
            let mut next_index = end;
            while next_index < bytes.len() && bytes[next_index].is_ascii_whitespace() {
                next_index += 1;
            }
            let starts_next = match text[next_index..].chars().next() {
                None => true,
                Some(next_char) => {
                    next_char.is_uppercase()
                        || next_char.is_ascii_digit()
                        || matches!(next_char, '"' | '\'' | '(' | '[' | '{' | '#')
                }
            };
            let decimal = byte == b'.'
                && index > 0
                && bytes[index - 1].is_ascii_digit()
                && bytes
                    .get(index + 1)
                    .is_some_and(|byte| byte.is_ascii_digit());
            let abbreviation = byte == b'.' && ends_abbreviation(text, index);
            let initialism = byte == b'.' && is_initialism_period(text, index);
            if starts_next && !decimal && !abbreviation && !initialism {
                if let Some((sentence_start, sentence_end)) = trimmed_range(text, start, end) {
                    ranges.push((sentence_start, sentence_end));
                }
                start = next_index;
                index = next_index;
                continue;
            }
        }
        index += 1;
    }
    if let Some((sentence_start, sentence_end)) = trimmed_range(text, start, bytes.len()) {
        ranges.push((sentence_start, sentence_end));
    }
    ranges
}

/// Whether the period at `dot` ends a known abbreviation.
///
/// The abbreviation must be a whole word: `he aims.` ends with `ms.` but is not
/// the abbreviation, so the character before the match must not be alphanumeric.
fn ends_abbreviation(text: &str, dot: usize) -> bool {
    let prefix = text[..=dot].to_ascii_lowercase();
    ABBREVIATIONS.iter().any(|abbreviation| {
        prefix
            .strip_suffix(abbreviation)
            .is_some_and(|head| !head.chars().next_back().is_some_and(char::is_alphanumeric))
    })
}

/// Whether the period at `dot` belongs to an initialism run such as `U.S.` or
/// `Ph.D.` rather than ending a sentence.
///
/// Two positions in a run are recognised:
///
/// - an **internal** period, where a short token ends just before the dot and
///   the text after it (past any whitespace) is a single ASCII letter followed
///   by another period (`U.` in `U.S.`, `Ph.` in `Ph.D.`);
/// - the **closing** period of a run that already contains an internal period,
///   when the next non-space byte starts a capital word, such as the `P` in
///   "U.S. President". A capital that is a *new* initialism (a single letter
///   plus a period, such as `P.`) is handled by the first case, not the second.
///
/// Only byte-level ASCII checks are used, so a non-ASCII byte simply fails the
/// guard and `dot` is never advanced past a char boundary.
fn is_initialism_period(text: &str, dot: usize) -> bool {
    let bytes = text.as_bytes();
    if bytes.get(dot) != Some(&b'.') {
        return false;
    }
    let mut token_start = dot;
    while token_start > 0 && bytes[token_start - 1].is_ascii_alphabetic() {
        token_start -= 1;
    }
    let token_len = dot - token_start;
    if !(1..=2).contains(&token_len) {
        return false;
    }
    if token_start > 0 && bytes[token_start - 1].is_ascii_alphanumeric() {
        return false;
    }
    let mut after = dot + 1;
    while after < bytes.len() && bytes[after].is_ascii_whitespace() {
        after += 1;
    }
    if after + 1 < bytes.len() && bytes[after].is_ascii_alphabetic() && bytes[after + 1] == b'.' {
        return true;
    }
    // Closing period: a single letter after a period (e.g. `S.` in `U.S.`),
    // followed by a capital word that continues the same sentence.
    if token_len == 1 && token_start >= 2 && bytes[token_start - 1] == b'.' {
        let internal_dot = token_start - 1;
        let mut run_start = internal_dot;
        while run_start > 0 && bytes[run_start - 1].is_ascii_alphabetic() {
            run_start -= 1;
        }
        return (1..=2).contains(&(internal_dot - run_start))
            && (run_start == 0 || !bytes[run_start - 1].is_ascii_alphanumeric())
            && after < bytes.len()
            && bytes[after].is_ascii_uppercase();
    }
    false
}

/// Whether the punctuation at `index` sits between digits.
fn is_numeric_break(bytes: &[u8], index: usize) -> bool {
    let previous = index > 0 && bytes[index - 1].is_ascii_digit();
    let next = bytes[index + 1..]
        .iter()
        .find(|byte| !byte.is_ascii_whitespace())
        .is_some_and(|byte| byte.is_ascii_digit());
    previous && next
}

#[cfg(test)]
mod tests;
