//! Read selection and read-from-here.
//!
//! A reader is handed text and a place to start: either the current selection
//! or the pointer position. It walks the text one sentence at a time, remembers
//! where it is, and can be stopped and resumed without losing the place. See
//! [`../../../docs/READING_POSITION.md`](../../../docs/READING_POSITION.md).
//!
//! Four rules hold:
//!
//! - **Reading starts from an explicit request.** [`ReadingCursor::select`] and
//!   [`ReadingCursor::from_here`] are the only ways into [`ReadState::Reading`];
//!   the cursor never starts on its own.
//! - **Stop is always available and always wins.** [`ReadingCursor::stop`] from
//!   [`ReadState::Reading`] moves to [`ReadState::Stopped`], and stop from any
//!   other state is a no-op rather than a panic.
//! - **Position survives an interruption.** Stopping keeps the offset;
//!   [`ReadingCursor::resume`] continues from it. The position is a byte offset
//!   into the caller's text, not a copy of the text.
//! - **Only positions and counts leave the reader.** The cursor stores offsets
//!   and a length, never content, and its `Debug` output is offsets only.
//!
//! Sentence boundaries are heuristic: a `.`, `!`, `?`, newline, or a CJK full
//! stop ends a sentence when the next character is whitespace or the end of the
//! text. A period inside a number (`3.14`) does not end one; a period after an
//! abbreviation (`Dr.`) does. The caller that needs exact boundaries should
//! chunk with `curbcut-understand`'s text chunker first.

/// A half-open byte range in a text.
///
/// `end` is clamped up to `start`, so a reversed range is the empty range at
/// `start` rather than a panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextSpan {
    /// First byte of the range.
    pub start: usize,
    /// One past the last byte of the range.
    pub end: usize,
}

impl TextSpan {
    /// Creates a span, clamping `end` up to `start`.
    pub const fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end: if end < start { start } else { end },
        }
    }

    /// The number of bytes in the span.
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    /// Whether the span covers no bytes.
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Whether `offset` falls inside the span.
    pub const fn contains(self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }
}

/// Why a read could not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
    /// The selection covered no bytes, so there is nothing to read.
    EmptySelection,
    /// The requested start ran past the end of the text.
    PastEnd {
        /// The offset that was asked for.
        requested: usize,
        /// The length of the text.
        len: usize,
    },
}

/// Where a reader is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReadState {
    /// Nothing has been requested.
    #[default]
    Idle,
    /// Reading is in progress.
    Reading,
    /// Stopped, with the position kept.
    Stopped,
    /// The target ran out.
    Done,
}

/// A position-aware reader over one text.
///
/// The caller owns the text and its lifetime; the cursor holds a length and
/// offsets only. Every method is total: a text shorter than the recorded length
/// ends the read rather than panicking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingCursor {
    /// The length of the text the cursor was created with.
    total: usize,
    /// Where the current read started.
    origin: usize,
    /// One past the last byte the current read may reach.
    limit: usize,
    /// The next byte to read.
    position: usize,
    state: ReadState,
}

impl ReadingCursor {
    /// Creates an idle cursor over a text of `len` bytes.
    pub fn new(len: usize) -> Self {
        Self {
            total: len,
            origin: 0,
            limit: len,
            position: 0,
            state: ReadState::Idle,
        }
    }

    /// The length of the text.
    pub const fn len(&self) -> usize {
        self.total
    }

    /// Whether the text is empty.
    pub const fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// The current state.
    pub const fn state(&self) -> ReadState {
        self.state
    }

    /// The next byte to read.
    pub const fn position(&self) -> usize {
        self.position
    }

    /// Where the current read started.
    pub const fn origin(&self) -> usize {
        self.origin
    }

    /// One past the last byte the current read may reach.
    pub const fn limit(&self) -> usize {
        self.limit
    }

    /// Bytes left before the current target ends.
    pub fn remaining(&self) -> usize {
        self.limit.saturating_sub(self.position)
    }

    /// Whether a read is in progress.
    pub const fn is_reading(&self) -> bool {
        matches!(self.state, ReadState::Reading)
    }

    /// Whether the reader was stopped and can be resumed.
    pub const fn is_stopped(&self) -> bool {
        matches!(self.state, ReadState::Stopped)
    }

    /// Whether the current target ran out.
    pub const fn is_done(&self) -> bool {
        matches!(self.state, ReadState::Done)
    }

    /// Starts reading a selection.
    ///
    /// The read is limited to the selection: it stops at the selection's end
    /// even if the text continues.
    pub fn select(&mut self, span: TextSpan) -> Result<(), ReadError> {
        if span.is_empty() {
            return Err(ReadError::EmptySelection);
        }
        if span.end > self.total {
            return Err(ReadError::PastEnd {
                requested: span.end,
                len: self.total,
            });
        }
        self.origin = span.start;
        self.limit = span.end;
        self.position = span.start;
        self.state = ReadState::Reading;
        Ok(())
    }

    /// Starts reading from `offset` to the end of the text.
    pub fn from_here(&mut self, offset: usize) -> Result<(), ReadError> {
        if offset > self.total {
            return Err(ReadError::PastEnd {
                requested: offset,
                len: self.total,
            });
        }
        self.origin = offset;
        self.limit = self.total;
        self.position = offset;
        self.state = ReadState::Reading;
        Ok(())
    }

    /// Returns the next sentence and advances past it.
    ///
    /// Returns `None` when the target is exhausted, and moves to
    /// [`ReadState::Done`]. From any state other than [`ReadState::Reading`] it
    /// returns `None` without changing the position.
    pub fn next_sentence(&mut self, text: &str) -> Option<TextSpan> {
        if !self.is_reading() {
            return None;
        }
        if self.position >= self.limit {
            self.state = ReadState::Done;
            return None;
        }
        let Some(slice) = text.get(self.position..self.limit) else {
            self.state = ReadState::Done;
            return None;
        };
        let end = sentence_end(slice).max(1);
        let span = TextSpan::new(self.position, self.position + end);
        self.position = span.end;
        if self.position >= self.limit {
            self.state = ReadState::Done;
        }
        Some(span)
    }

    /// Stops reading, keeping the position.
    ///
    /// From [`ReadState::Idle`] or [`ReadState::Done`] this changes nothing.
    pub fn stop(&mut self) {
        if self.is_reading() {
            self.state = ReadState::Stopped;
        }
    }

    /// Resumes a stopped read from the kept position.
    pub fn resume(&mut self) {
        if self.is_stopped() {
            self.state = if self.position >= self.limit {
                ReadState::Done
            } else {
                ReadState::Reading
            };
        }
    }

    /// Starts the current target again from its origin.
    pub fn restart(&mut self) {
        if self.is_reading() || self.is_stopped() || self.is_done() {
            self.position = self.origin;
            self.state = ReadState::Reading;
        }
    }
}

/// The index one past the end of the sentence that starts at byte 0 of `slice`.
///
/// A newline or a CJK full stop always ends a sentence. `.`, `!`, and `?` end
/// one only when the next character is whitespace or the slice ends. Returns
/// `slice.len()` when there is no terminator.
fn sentence_end(slice: &str) -> usize {
    for (at, ch) in slice.char_indices() {
        let consumed = at + ch.len_utf8();
        let boundary = if is_hard_terminator(ch) {
            true
        } else if is_terminator(ch) {
            let rest = &slice[consumed..];
            rest.is_empty() || rest.chars().next().is_some_and(char::is_whitespace)
        } else {
            false
        };
        if boundary {
            return consumed;
        }
    }
    slice.len()
}

/// Whether a character always ends a sentence, even without a space after it.
fn is_hard_terminator(ch: char) -> bool {
    matches!(ch, '\n' | '。' | '！' | '？' | '।')
}

/// Whether a character can end a sentence.
fn is_terminator(ch: char) -> bool {
    matches!(ch, '.' | '!' | '?' | '\n' | '。' | '！' | '？' | '।')
}

#[cfg(test)]
mod tests;
