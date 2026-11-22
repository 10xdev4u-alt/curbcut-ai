# Typography

`crates/curbcut-adapt/src/typography.rs` adds the style around a reflow: which
font family survives fallback, how tall a line box is, how a line aligns inside
the measure, and whether the palette meets WCAG AA. `CC-057`.

The wrap itself belongs to the layout solver: `layout::reflow` breaks measured
words into lines and never splits a word. This module consumes that result and
adds style, so the two never disagree about where a line breaks.

## Font fallback

A `FontStack` is an ordered list of `FontFamily` names. `resolve(available)`
returns the first family the platform has, and `None` when the stack has nothing
available. The caller then falls back to the platform default.

A stack never invents a family and never panics: an empty stack, an empty
available set, and a stack of unavailable families all return `None`. The
resolved family is reported on the layout, so a report can say which font was
actually used.

## Line box

`TextStyle::line_box` is the requested line height (`line_height_px`) raised to
`size_px`. A box shorter than the font clips ascenders and descenders, so a
profile cannot produce one. A profile that asks for a taller box gets exactly
that.

## Alignment

`Alignment` is applied **after** the wrap, as an indent inside the measure:

- `Start` indents 0.
- `Center` indents half the slack.
- `End` indents all of it.

Alignment therefore cannot change which words share a line. An overflowing line
has no slack, so its indent is 0.

## Contrast

`contrast_ratio(foreground, background)` is the WCAG relative-luminance ratio,
symmetric and bounded to `1.0..=21.0`. `meets_aa(ratio, large_text)` applies the
AA thresholds: 4.5 for normal text and 3.0 for large text. Large text is 24 px or
larger, or 19 px and bold.

`style_reflow` reports both the ratio and the verdict, so a palette that fails
AA is visible rather than shipped. A `NaN` ratio never meets the threshold.

## Rules

1. **A font is resolved before it is used.** No family is invented.
2. **A line box is never shorter than the font.**
3. **Alignment is an indent, not a re-wrap.**
4. **Contrast is measured, not assumed.**

## Privacy

The module holds geometry, colors, and family names only. It never holds text:
`StyledLine` carries word indices, not the words. A debug dump therefore cannot
leak screen content.
