# Reading position

`crates/curbcut-tts/src/read.rs` reads the current selection, or reads
sequentially from the pointer, and remembers the place across an interruption.
`CC-078`.

## Platform support

The cursor is platform-agnostic and holds offsets only. The adapter supplies
the text and the start:

- **macOS** exposes the selected text through the accessibility APIs
  (`AXSelectedText`), so selection reading works in most apps.
- **Windows** exposes it through UI Automation (`UiaGetSelectedText`), again in
  most apps.
- **Linux** exposes it through AT-SPI (`atspi_get_selected_text`). An app that
  exposes no selection falls back to read-from-here.

Selection reading depends on the focused app cooperating. Where an app exposes
no selection, the adapter starts from the pointer position instead and says so,
rather than reporting a selection that was never read.

## Rules

1. **Reading starts from an explicit request.** `ReadingCursor::select` and
   `ReadingCursor::from_here` are the only ways into `ReadState::Reading`. The
   cursor never starts on its own.
2. **Stop is always available and always wins.** `stop` from `ReadState::Reading`
   moves to `ReadState::Stopped`. Stop from any other state is a no-op, never a
   panic, so a stop key that arrives late or twice is safe.
3. **Position survives an interruption.** Stopping keeps the byte offset;
   `resume` continues from it. The position is an offset into the caller's text,
   not a copy of the text.
4. **Only positions and counts leave the reader.** The cursor stores a length
   and offsets, never content, and its `Debug` output is offsets only. A
   selection's text is never logged.

## Boundaries

A newline or a CJK full stop always ends a sentence. `.`, `!`, and `?` end
one when the next character is whitespace or the end of the text. The rule is
deliberately conservative:

- A period inside a number does not end a sentence (`3.14` stays whole).
- A period after an abbreviation does (`Dr. Smith` splits). The caller that
  needs exact boundaries should chunk with `curbcut-understand` first.
- A terminator followed by a closing quote does not split, so `"Hi." Next` is
  one chunk.

## Lifecycle

`select` limits the read to the selection and stops at its end even when the
text continues. `from_here` reads to the end of the text. `next_sentence`
returns the next sentence and advances; it returns `None` and moves to
`ReadState::Done` when the target is exhausted. A text shorter than the
recorded length ends the read instead of panicking.

`restart` returns to the origin of the current target. `select` after a stop
replaces the target, which is how a new selection takes over from a paused read.
