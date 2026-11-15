# Jargon dictionary and acronym expansion

`crates/curbcut-understand/src/jargon.rs` holds an open, versioned jargon
dictionary and an acronym expander. `CC-043`.

## Rules

1. **Every entry has a source and a locale.** `JargonEntry::new` takes both a
   `Locale` and an `EntrySource`; there is no entry without one.
2. **Users can add and override entries.** `JargonDictionary::add` replaces any
   entry with the same locale and term and returns what it replaced, so a
   `EntrySource::User` entry overrides a built-in. `EntrySource::priority` is
   the tie-break rule, with user above community above built-in.
3. **Unknown jargon is flagged, not guessed.** `lookup` returns `None` for an
   unknown term and an `AcronymMatch` for an unknown acronym carries no
   expansion. The caller decides what to show; the dictionary never invents one.

## Types

- `Locale` — a tag normalised to lowercase for matching. `Default` is `en`.
- `EntrySource { BuiltIn, Community, User }` — with `as_str` and `priority`.
- `JargonKind { Term, Acronym }` — a word or phrase, or an acronym.
- `JargonEntry` — `term`, `expansion`, `locale`, `source`, `kind`.
- `JargonDictionary` — `new`, `with_builtin_defaults`, `len`, `is_empty`,
  `entries`, `add`, `lookup`, `is_known`, `remove`, `clear`.
- `AcronymMatch` — `token`, `start`, `end`, `expansion`, `source`, `is_known`.
- `is_acronym_token`, `expand_acronyms`, `unknown_acronyms`.

## Acronym detection

`is_acronym_token` accepts two to eight ASCII alphanumeric characters with at
least one letter and no lowercase. `API`, `ROI`, and `HTTP2` qualify; `42`,
`api`, and `APIs` do not. `expand_acronyms` walks the text, splits runs on
non-alphanumerics, and reports byte offsets, so the caller can underline the
token in place.

`unknown_acronyms` lists each unknown acronym once, in order of appearance, so
a review surface can flag what the dictionary does not know.

## Adding entries

`with_builtin_defaults` seeds a small starter set. A locale-specific entry does
not collide with the same term in another locale. `remove` takes an entry out;
`clear` drops the whole dictionary.

## Privacy

The dictionary stores terms and expansions, which are dictionary data rather
than screen content. `expand_acronyms` is pure: it returns offsets and
expansions and retains nothing it read. Nothing here logs the text it scans.
The workspace canary scan covers this crate.
