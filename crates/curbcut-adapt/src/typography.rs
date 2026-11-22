//! Typography: font fallback, profile-controlled style, and contrast.
//!
//! The layout solver breaks measured words into lines; this module adds the
//! style around them: which font family survives fallback, how tall a line box
//! is, how a line aligns inside the measure, and whether the palette meets
//! WCAG AA. See
//! [`../../../docs/TYPOGRAPHY.md`](../../../docs/TYPOGRAPHY.md).
//!
//! Four rules hold:
//!
//! - **A font is resolved before it is used.** [`crate::typography::FontStack::resolve`]
//!   returns the first family the platform has, and `None` when the stack has
//!   nothing available. The caller then falls back to the platform default; the
//!   module never panics and never invents a family.
//! - **A line box is never shorter than the font.**
//!   [`crate::typography::TextStyle::line_box`] is the requested line height
//!   raised to the font size, so a profile cannot squeeze text into a box that
//!   clips it.
//! - **Alignment is an indent, not a re-wrap.** Start, center, and end align by
//!   padding the line inside the measure. The line width is unchanged.
//! - **Contrast is measured, not assumed.** [`crate::typography::contrast_ratio`]
//!   is the WCAG relative-luminance ratio and
//!   [`crate::typography::meets_aa`] applies the AA threshold for the text size,
//!   so a palette that fails is reported rather than shipped.
//!
//! The module holds geometry, colors, and family names only. It never holds
//! text, so a debug dump cannot leak screen content.

use std::collections::BTreeSet;

use crate::layout::{reflow, Word};

/// An sRGB color, 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    /// Pure black.
    pub const BLACK: Self = Self::new(0, 0, 0);
    /// Pure white.
    pub const WHITE: Self = Self::new(255, 255, 255);

    /// Creates a color.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// The red channel.
    pub const fn r(self) -> u8 {
        self.r
    }

    /// The green channel.
    pub const fn g(self) -> u8 {
        self.g
    }

    /// The blue channel.
    pub const fn b(self) -> u8 {
        self.b
    }
}

/// The WCAG relative luminance of a color, in `0.0..=1.0`.
fn relative_luminance(color: Color) -> f64 {
    let channel = |value: u8| {
        let srgb = f64::from(value) / 255.0;
        if srgb <= 0.039_28 {
            srgb / 12.92
        } else {
            ((srgb + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

/// The WCAG contrast ratio between two colors, in `1.0..=21.0`.
///
/// The ratio is symmetric, so the order of the arguments does not matter. A
/// color against itself is `1.0`; black against white is `21.0`.
pub fn contrast_ratio(foreground: Color, background: Color) -> f64 {
    let first = relative_luminance(foreground);
    let second = relative_luminance(background);
    let (lighter, darker) = if first >= second {
        (first, second)
    } else {
        (second, first)
    };
    (lighter + 0.05) / (darker + 0.05)
}

/// Whether a contrast ratio meets WCAG AA.
///
/// Large text is 18 pt (24 px) or larger, or 14 pt (about 19 px) and bold; the
/// large-text threshold is 3.0 and the normal threshold is 4.5. A `NaN` ratio
/// does not meet the threshold.
pub fn meets_aa(ratio: f64, large_text: bool) -> bool {
    let threshold = if large_text { 3.0 } else { 4.5 };
    ratio >= threshold
}

/// How a line aligns inside the measure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Alignment {
    /// The start edge.
    #[default]
    Start,
    /// Centered, with equal indent on both sides.
    Center,
    /// The end edge.
    End,
}

/// A font family name.
///
/// A family name is not screen content, so it is safe to log.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontFamily(String);

impl FontFamily {
    /// Creates a family from its name.
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }

    /// The family name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for FontFamily {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// An ordered font stack: the first family the platform has wins.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct FontStack(Vec<FontFamily>);

impl FontStack {
    /// Creates a stack from an ordered list.
    pub fn new(families: Vec<FontFamily>) -> Self {
        Self(families)
    }

    /// Creates a stack from family names.
    pub fn from_names(names: &[&str]) -> Self {
        Self(names.iter().map(|name| FontFamily::new(name)).collect())
    }

    /// The families in preference order.
    pub fn families(&self) -> &[FontFamily] {
        &self.0
    }

    /// The number of families.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The first family the platform has, or `None` when none is available.
    pub fn resolve(&self, available: &BTreeSet<FontFamily>) -> Option<FontFamily> {
        self.0
            .iter()
            .find(|candidate| available.contains(*candidate))
            .cloned()
    }
}

/// A profile-controlled text style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextStyle {
    /// The font stack, in preference order.
    pub font: FontStack,
    /// The font size in pixels.
    pub size_px: u32,
    /// The requested line height in pixels.
    pub line_height_px: u32,
    /// Whether the text is bold, which lowers the AA large-text size.
    pub bold: bool,
    /// The alignment inside the measure.
    pub alignment: Alignment,
    /// The space between words in pixels.
    pub space_px: u32,
    /// The text color.
    pub foreground: Color,
    /// The background color.
    pub background: Color,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: FontStack::default(),
            size_px: 16,
            line_height_px: 16,
            bold: false,
            alignment: Alignment::default(),
            space_px: 4,
            foreground: Color::BLACK,
            background: Color::WHITE,
        }
    }
}

impl TextStyle {
    /// Whether the size and weight count as WCAG large text.
    pub fn is_large_text(&self) -> bool {
        self.size_px >= 24 || (self.bold && self.size_px >= 19)
    }

    /// The line box height: the requested line height raised to the font size.
    ///
    /// A box shorter than the font clips ascenders and descenders, so the
    /// effective height is never below `size_px`.
    pub fn line_box(&self) -> u32 {
        self.line_height_px.max(self.size_px)
    }
}

/// A laid-out line: which words it holds, how wide it is, and its indent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledLine {
    /// Indices into the input word slice, in order.
    pub words: Vec<usize>,
    /// Advance width including the spaces between words.
    ///
    /// `u64`, matching the layout solver, so a line of very wide words cannot
    /// saturate and hide an overflow.
    pub width: u64,
    /// The indent alignment produces inside the measure.
    pub indent: u32,
}

/// The outcome of a styled reflow.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledLayout {
    /// Lines in reading order.
    pub lines: Vec<StyledLine>,
    /// The effective line box height in pixels.
    pub line_height_px: u32,
    /// The measure the lines were broken against.
    pub measure: u32,
    /// Indices into `lines` whose width exceeds the measure.
    pub overflowing: Vec<usize>,
    /// The font family that survived fallback, if any.
    pub resolved_font: Option<FontFamily>,
    /// The contrast ratio of the style's palette.
    pub contrast_ratio: f64,
    /// Whether that ratio meets WCAG AA for the style's size and weight.
    pub meets_aa: bool,
}

impl StyledLayout {
    /// How many lines were produced.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether nothing was produced.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Every input word appears exactly once, in order.
    pub fn preserves_words(&self, word_count: usize) -> bool {
        let seen: Vec<usize> = self
            .lines
            .iter()
            .flat_map(|line| line.words.iter().copied())
            .collect();
        seen == (0..word_count).collect::<Vec<usize>>()
    }

    /// Whether any line is clipped: wider than the measure while holding more
    /// than one word.
    ///
    /// A single word wider than the measure is reported as overflowing rather
    /// than clipped, which is what the layout solver guarantees.
    pub fn has_clipped_line(&self) -> bool {
        self.lines
            .iter()
            .any(|line| line.width > u64::from(self.measure) && line.words.len() > 1)
    }
}

/// Breaks `words` into styled lines no wider than `measure`.
///
/// The wrap is the layout solver's: a word is never split, so a word wider than
/// the measure occupies a line of its own and that line is reported in
/// `overflowing` rather than clipped. Alignment is applied afterwards as an
/// indent, so it cannot change which words share a line.
///
/// `available` is the set of families the platform has; the stack resolves
/// against it and the layout reports the winner.
pub fn style_reflow(
    words: &[Word],
    style: &TextStyle,
    available: &BTreeSet<FontFamily>,
    measure: u32,
) -> StyledLayout {
    let reflowed = reflow(words, measure, style.space_px);
    let lines = reflowed
        .lines
        .into_iter()
        .map(|line| StyledLine {
            indent: indent_for(style.alignment, measure, line.width),
            words: line.words,
            width: line.width,
        })
        .collect();

    let ratio = contrast_ratio(style.foreground, style.background);
    StyledLayout {
        lines,
        line_height_px: style.line_box(),
        measure,
        overflowing: reflowed.overflowing_lines,
        resolved_font: style.font.resolve(available),
        contrast_ratio: ratio,
        meets_aa: meets_aa(ratio, style.is_large_text()),
    }
}

/// The indent an alignment produces for a line of `width` inside `measure`.
///
/// The slack is computed in `u64` and cannot exceed `measure`, so the narrowing
/// to `u32` is lossless.
fn indent_for(alignment: Alignment, measure: u32, width: u64) -> u32 {
    let slack = u64::from(measure).saturating_sub(width);
    let indent = match alignment {
        Alignment::Start => 0,
        Alignment::Center => slack / 2,
        Alignment::End => slack,
    };
    u32::try_from(indent).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;
