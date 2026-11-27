# Text chunking

`crates/curbcut-understand/src/text_chunking.rs` splits plain text into chunks
for reading and highlighting. `CC-048`.

## Rules

1. **The order is the reading order.** Chunks are emitted top to bottom with
   ids sequential from zero, so a chunk id is stable for highlighting.
2. **A table stays whole.** Consecutive table lines become one `table` chunk
   and are never split across chunks, even when a cell holds a sentence.
3. **No mid-word splits.** A period inside a number (`3.14`), a whole-word
   abbreviation (`e.g.`, `Dr.`), or an initialism (`U.S.`, `A.B.`, `Ph.D.`) does
   not end a sentence, and a clause break needs a following space, so `1,000`
   and `12:30` stay intact. A word that merely ends in an abbreviation string
   (`he aims.`) is still a boundary. The initialism rule is conservative: a
   short capitalised word followed by a single letter and a period (`OK. A.`)
   may stay in one chunk rather than be split into two sentences.

Only `|`-delimited tables are recognised; a row like `a | b` is read as
paragraph text.

## Kinds

- `heading` — a `#` heading line or a short all-caps line.
- `list-item` — a `-`, `*`, `+`, bullet, `1.`, or `1)` line.
- `table` — a whole run of `|`-delimited lines, including the separator row.
- `sentence` — a sentence within a paragraph, ending at `.`, `!`, or `?` when
   the next non-space character starts a new sentence.
- `clause` — a part of a sentence split on `,`, `;`, or `:` followed by a space.

## API

- `chunk_text(source) -> Vec<TextChunk>` — headings, list items, tables, and
  sentences. Ids start at zero.
- `split_clauses(source, start, end, first_id) -> Vec<TextChunk>` — clauses
  within a byte range of `source`; ids continue from `first_id`.
- `TextChunk` — `id`, `kind`, `start`, `end`, `len`, `is_empty`, and
  `text(source)`, which borrows the chunk's text instead of copying it.
- `ChunkKind` — `Heading`, `ListItem`, `Table`, `Sentence`, `Clause`, each with
  a stable `as_str`.

A chunk is a half-open byte range, so `chunk.text(source)` is a `&str` borrow
and the module never copies content it was given.

## Privacy

The module reads text the caller passes and returns ranges. It never logs the
text, so a chunk dump cannot leak what is on screen.
