# Reading level scoring

`crates/curbcut-understand/src/reading.rs` scores a text block with Flesch
reading ease, Flesch-Kincaid grade, SMOG, and ARI, plus syllable count and
word-length bands. `CC-039`.

## Scores

| Score | Formula | Range |
| --- | --- | --- |
| Flesch reading ease | `206.835 - 1.015·(words/sentences) - 84.6·(syllables/words)` | higher is easier |
| Flesch-Kincaid grade | `0.39·(words/sentences) + 11.8·(syllables/words) - 15.59` | US grade level |
| SMOG | `1.043·√(polysyllables·30/sentences) + 3.1291` | US grade level |
| ARI | `4.71·(characters/words) + 0.5·(words/sentences) - 21.43` | US grade level |

`polysyllables` counts words with three or more syllables. `characters` counts
alphanumeric characters, excluding spaces and punctuation. Syllables use the
vowel-group heuristic already in the module; it is an estimate, not a dictionary
lookup.

SMOG needs about 30 sentences. `smog` returns `None` for fewer than three
sentences rather than reporting a grade it cannot support.

## Script support

`classify_script` inspects the letters: `Empty` (no letters), `Latin`, `Mixed`,
or `NonLatin`. Only `Latin` is supported. A non-Latin or mixed text is flagged
as unsupported and every score is `None` — it is never mis-scored. Latin-1
supplement and Latin Extended ranges count as Latin.

## Word bands

`WordBands` is a histogram of word lengths by syllable count: `short` (1-2),
`medium` (3), and `long` (4+). `total` is the number of words banded.

## Cache

`ScoreCache` is a bounded FIFO of `ReadingScore` values keyed by
`content_hash`. `score(text)` returns the cached value on a hit and computes,
caches, and returns it on a miss. When full, the oldest entry is evicted. The
default limit is 64.

## Privacy

The cache stores scores, counts, and a non-cryptographic identity hash. It
never stores the text, screen content, OCR output, or accessibility strings.
The hash is an identity key, not a security boundary. See
[`docs/HASHES.md`](HASHES.md). The workspace canary scan covers this crate.
