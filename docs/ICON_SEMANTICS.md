# Icon semantics

`crates/curbcut-understand/src/icon_semantics.rs` turns the evidence around one
icon into a plain "what this does" label of at most eight words. `CC-049`.

The caption model describes a picture; the platform's accessibility label
describes the control. Icon semantics combines the accessible label, the
caption, and the labels of neighbouring elements, and reports how confident it
is.

## Rules

1. **The accessible label wins.** When the platform reports a label, it is the
   author's own description. It is trimmed, collapsed, and used as-is, and the
   result is never uncertain.
2. **A caption is a fallback, not an answer.** A caption is used only when its
   confidence is at least `min_caption_confidence` (default `0.6`). A caption
   that only names the medium — `icon`, `image`, `photo`, `picture`, `logo`,
   `symbol`, `graphic`, `illustration`, or `button` — never counts as a certain
   answer. A leading `a`, `an`, or `the` is ignored, so `a logo` and `the icon`
   are generic too.
3. **A neighbour is a hint.** A neighbouring element's label is used only when
   neither the accessible label nor a usable caption is available. It is always
   marked uncertain, because a neighbour may not describe the icon.
4. **The label is short.** Every label is capped at `MAX_LABEL_WORDS` (8) words,
   so an icon label never becomes a sentence. A caption prefix that names the
   medium — `a photo of`, `an image of`, `a picture of` — is dropped only at a
   word boundary, so `photo office` keeps its `photo`.

The order is fixed: accessible label, confident non-generic caption, neighbour,
any caption, then `unlabeled icon`.

## Uncertainty

`IconSemantics::is_uncertain` is false only for an accessible label or a
confident, non-generic caption. A neighbour's label, a generic or low-confidence
caption, and the `unlabeled icon` fallback are all uncertain, so the overlay can
say "this might be…" instead of asserting a guess.

`IconSemanticsConfig::clamped` bounds `min_caption_confidence` to `0.0..=1.0`
and falls back to the default on `NaN`.

## API

- `resolve(input, config)` — resolves one icon.
- `IconInput` — `a11y_label`, `caption`, `caption_confidence`, `neighbours`.
- `IconSemantics` — `label`, `source`, `is_uncertain`, `redacted`.
- `IconLabelSource` — `A11y`, `Caption`, `Neighbour`, `Generic`, with `as_str`.
- `IconSemanticsConfig` — `min_caption_confidence`, `clamped`.
- `RedactedIconSemantics` — `label_len`, `source`, `uncertain`.

## Privacy

The label is derived from screen content. Neither `IconSemantics` nor
`IconInput` formats it with `Debug`: both debug views carry lengths, the source,
the confidence, and the uncertainty flag, never the text. The module logs
nothing.
