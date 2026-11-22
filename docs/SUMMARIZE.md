# Extractive summarization

`crates/curbcut-understand/src/summarize.rs` turns a long document into a short
list of key points, each cited back to its place in the source. `CC-047`.

## Scope

The summarizer is **extractive**. It selects sentences from the document and
never writes a new one, so a summary cannot state a fact that the document does
not contain. An abstractive step — paraphrasing, merging, or generating — is a
model call and belongs to `CC-045`. That split is deliberate: extractive output
is deterministic, offline, and auditable, and it is the only kind that can be
checked against the source without a model in the loop.

## Rules

1. **A key point cites its source.** Every `KeyPoint` carries the half-open byte
   range `start..end` in the original document and the `chunk_id` of the
   sentence chunk it came from. `KeyPoint::cited` returns the exact source slice
   and `KeyPoint::citation` renders `chunk N, bytes A..B`, so a point can always
   be jumped back to.
2. **Reading order is restored.** Candidates are ranked by score, but the emitted
   points are sorted by `start`, so the summary never reorders the document.
   `Summary::reading_order_preserved` asserts the invariant.
3. **A near-duplicate is not a key point.** A candidate whose Jaccard word
   overlap with an already-selected point exceeds `MAX_REDUNDANCY` (0.8) is
   skipped. The comparison is against the selected points, in selection order.
4. **The metric is reported, not gated.** `SummaryMetrics` holds ROUGE-1 recall
   and ROUGE-L recall against the source. Nothing refuses to produce a summary
   because of them; a low score is information for the caller, not a failure.

## Scoring

`chunk_text` splits the document into headings, list items, tables, sentences,
and clauses. Only `ChunkKind::Sentence` chunks are candidates, so a heading or a
list item is never summarized as prose.

A sentence is a candidate when its word count is within
`SummaryOptions::min_words..=max_words` (default 4..=60). Word boundaries are
non-alphanumeric runs, lower-cased.

The score of a candidate is the **mean term frequency** of its words: the sum of
each word's count over the whole document's sentences, divided by the sentence's
word count. A frequent word repeated in one sentence does not inflate the score,
because the denominator is the sentence length. A sentence with no words is not
a candidate at all, rather than a zero-score one.

Ties break on the earlier `start`, so the result is deterministic for equal
scores.

## Output

`Summary` holds the selected points, the source's byte length, the options used,
and the metrics.

- `points` — the key points in reading order.
- `text` — the points joined by a single space.
- `compression_ratio` — summary bytes over source bytes, clamped to `1.0`; an
  empty source is `0.0`.
- `citations` — one citation per point, in reading order.
- `cited_sentences` — the source slice each point cites, skipping a range that
  does not fit.

`SummaryMetrics` reports `source_sentences`, `candidates`, `selected`,
`source_words`, `summary_words`, `rouge_1_recall`, and `rouge_l_recall`.

## ROUGE

Both metrics are **recall** against the source, so they answer "how much of the
source did the summary cover" rather than "how much of the summary was in the
source". Recall is the honest direction for an extractive summary: a summary is
expected to be much shorter than the document.

- `rouge_1_recall` — clipped unigram overlap divided by the reference's unigram
  count. `0.0` when the reference has no words.
- `rouge_l_recall` — the longest common subsequence over the reference's word
  count, computed with two rows, so the cost is `O(candidate * reference)` time
  and `O(reference)` memory. `0.0` when the reference has no words.

Because the summary is a selection of source sentences, ROUGE-L recall is
normally high; it is a coverage check, not a quality score.

## Edge cases

- An empty source, or a source with no candidate sentence, yields an empty
  summary with `selected == 0` and both ROUGE scores `0.0`.
- `max_points == 0` yields an empty summary even when candidates exist.
- `min_words > max_words` yields an empty summary.
- A document of one candidate sentence yields that sentence, `compression_ratio`
  equal to the sentence's share of the source.

## Privacy

The module keeps the selected sentences and the source's length; it does not
retain the whole document. It never logs a sentence. `KeyPoint` and `Summary`
implement `Debug` with lengths and offsets rather than the text, so a `{:?}` in
a log cannot leak a sentence. The workspace canary scan covers this crate.
