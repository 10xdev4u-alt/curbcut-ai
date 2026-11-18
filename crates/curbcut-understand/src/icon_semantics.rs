//! Icon semantics.
//!
//! [`resolve`] combines an icon's accessible label, its caption, and the labels
//! of its neighbours into one plain "what this does" label of at most
//! [`MAX_LABEL_WORDS`] words. See
//! [`../../../docs/ICON_SEMANTICS.md`](../../../docs/ICON_SEMANTICS.md).
//!
//! Four rules hold:
//!
//! - **The accessible label wins.** When the platform reports a label, it is
//!   the author's own description and is never second-guessed.
//! - **A caption is a fallback, not an answer.** A caption is used only when
//!   its confidence clears [`IconSemanticsConfig::min_caption_confidence`], and
//!   a generic caption never counts as a certain answer.
//! - **A neighbour is a hint.** A neighbour's label is used only when neither
//!   label nor caption is usable, and it is always marked uncertain.
//! - **The label is short.** Every label is capped at [`MAX_LABEL_WORDS`]
//!   words, so an icon label never becomes a sentence.

use std::fmt;

use curbcut_core::types::MAX_LABEL_WORDS;

/// The default minimum caption confidence.
pub const DEFAULT_MIN_CAPTION_CONFIDENCE: f32 = 0.6;

/// The label used when nothing describes the icon.
pub const UNLABELED_ICON: &str = "unlabeled icon";

/// Captions that describe the picture, not what it does.
const GENERIC_CAPTIONS: [&str; 9] = [
    "icon",
    "image",
    "graphic",
    "photo",
    "picture",
    "logo",
    "symbol",
    "illustration",
    "button",
];

/// Caption prefixes that name the medium rather than the action.
const MEDIUM_PREFIXES: [&str; 6] = [
    "a photo of",
    "an image of",
    "a picture of",
    "photo of",
    "image of",
    "picture of",
];

/// Where an icon's label came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconLabelSource {
    /// The platform's accessible label.
    A11y,
    /// The caption model.
    Caption,
    /// A neighbouring element's label.
    Neighbour,
    /// Nothing described the icon.
    Generic,
}

impl IconLabelSource {
    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::A11y => "a11y",
            Self::Caption => "caption",
            Self::Neighbour => "neighbour",
            Self::Generic => "generic",
        }
    }
}

/// Thresholds for [`resolve`].
#[derive(Debug, Clone, PartialEq)]
pub struct IconSemanticsConfig {
    /// The lowest caption confidence still read as a description.
    pub min_caption_confidence: f32,
}

impl Default for IconSemanticsConfig {
    fn default() -> Self {
        Self {
            min_caption_confidence: DEFAULT_MIN_CAPTION_CONFIDENCE,
        }
    }
}

impl IconSemanticsConfig {
    /// Clamps the threshold into `0.0..=1.0`, falling back to the default on
    /// `NaN`.
    pub fn clamped(mut self) -> Self {
        self.min_caption_confidence = if self.min_caption_confidence.is_nan() {
            DEFAULT_MIN_CAPTION_CONFIDENCE
        } else {
            self.min_caption_confidence.clamp(0.0, 1.0)
        };
        self
    }
}

/// A plain "what this does" label for one icon.
#[derive(Clone, PartialEq, Eq)]
pub struct IconSemantics {
    label: String,
    source: IconLabelSource,
    uncertain: bool,
}

impl IconSemantics {
    /// Creates a label.
    pub fn new(label: impl Into<String>, source: IconLabelSource, uncertain: bool) -> Self {
        Self {
            label: label.into(),
            source,
            uncertain,
        }
    }

    /// The label, at most [`MAX_LABEL_WORDS`] words.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Where the label came from.
    pub const fn source(&self) -> IconLabelSource {
        self.source
    }

    /// Whether the label is a guess.
    pub const fn is_uncertain(&self) -> bool {
        self.uncertain
    }

    /// A content-free view: the label length, the source, and the uncertainty.
    pub const fn redacted(&self) -> RedactedIconSemantics {
        RedactedIconSemantics {
            label_len: self.label.len(),
            source: self.source,
            uncertain: self.uncertain,
        }
    }
}

impl fmt::Debug for IconSemantics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IconSemantics")
            .field("label_len", &self.label.len())
            .field("source", &self.source)
            .field("uncertain", &self.uncertain)
            .finish()
    }
}

/// A content-free view of [`IconSemantics`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RedactedIconSemantics {
    /// Length of the label in bytes.
    pub label_len: usize,
    /// Where the label came from.
    pub source: IconLabelSource,
    /// Whether the label is a guess.
    pub uncertain: bool,
}

/// The evidence available for one icon.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IconInput<'a> {
    /// The platform's accessible label, if it reported one.
    pub a11y_label: Option<&'a str>,
    /// The caption model's description, if it produced one.
    pub caption: Option<&'a str>,
    /// The caption model's confidence, from `0.0` to `1.0`.
    pub caption_confidence: f32,
    /// Labels of the elements around the icon, in reading order.
    pub neighbours: &'a [&'a str],
}

impl<'a> IconInput<'a> {
    /// Creates an input.
    pub const fn new(
        a11y_label: Option<&'a str>,
        caption: Option<&'a str>,
        caption_confidence: f32,
        neighbours: &'a [&'a str],
    ) -> Self {
        Self {
            a11y_label,
            caption,
            caption_confidence,
            neighbours,
        }
    }
}

/// Resolves one icon's label.
///
/// The order is fixed: a usable accessible label, then a confident non-generic
/// caption, then a neighbour's label, then any caption, then
/// [`UNLABELED_ICON`].
pub fn resolve(input: &IconInput<'_>, config: &IconSemanticsConfig) -> IconSemantics {
    if let Some(label) = clean(input.a11y_label.unwrap_or("")) {
        return IconSemantics::new(clip_words(&label), IconLabelSource::A11y, false);
    }

    let caption = input
        .caption
        .map(simplify_caption)
        .filter(|text| !text.is_empty());
    let confident = !input.caption_confidence.is_nan()
        && input.caption_confidence >= config.min_caption_confidence;

    if confident {
        if let Some(text) = caption.as_deref() {
            if !is_generic(text) {
                return IconSemantics::new(clip_words(text), IconLabelSource::Caption, false);
            }
        }
    }

    if let Some(neighbour) = input.neighbours.iter().find_map(|label| clean(label)) {
        return IconSemantics::new(clip_words(&neighbour), IconLabelSource::Neighbour, true);
    }

    if let Some(text) = caption.as_deref() {
        return IconSemantics::new(clip_words(text), IconLabelSource::Caption, true);
    }

    IconSemantics::new(UNLABELED_ICON, IconLabelSource::Generic, true)
}

/// Trims a label and collapses whitespace, returning `None` when nothing is
/// left.
fn clean(value: &str) -> Option<String> {
    let text = collapse_whitespace(value);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// Collapses whitespace and caps a label at [`MAX_LABEL_WORDS`] words.
fn clip_words(text: &str) -> String {
    text.split_whitespace()
        .take(MAX_LABEL_WORDS)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Collapses whitespace.
fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Drops the medium prefix and collapses whitespace.
fn simplify_caption(caption: &str) -> String {
    let lowered = caption.to_ascii_lowercase();
    let cut = MEDIUM_PREFIXES
        .iter()
        .find_map(|prefix| lowered.strip_prefix(prefix).map(|_| prefix.len()));
    let body = match cut {
        Some(len) => &caption[len..],
        None => caption,
    };
    collapse_whitespace(body)
}

/// Whether a caption names the medium rather than the action.
fn is_generic(text: &str) -> bool {
    let normalized = text
        .trim()
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_ascii_lowercase();
    GENERIC_CAPTIONS.contains(&normalized.as_str())
}

#[cfg(test)]
mod tests;
