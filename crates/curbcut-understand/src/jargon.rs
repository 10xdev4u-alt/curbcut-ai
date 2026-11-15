//! Jargon dictionary and acronym expansion.
//!
//! The dictionary is open and versioned: every entry carries a [`Locale`] and
//! an [`EntrySource`], so a user override is never confused with a built-in.
//! See [`../../../docs/JARGON.md`](../../../docs/JARGON.md).
//!
//! Three rules hold:
//!
//! - **Every entry has a source and a locale.** [`JargonEntry::new`] takes both;
//!   there is no entry without one.
//! - **Users can add and override entries.** [`JargonDictionary::add`] replaces
//!   any entry with the same locale and term and returns what it replaced.
//! - **Unknown jargon is flagged, not guessed.** A lookup returns `None` and an
//!   [`AcronymMatch`] carries no expansion; the caller decides what to show.
//!
//! The dictionary stores terms and expansions, which are dictionary data. The
//! expander stores nothing: [`expand_acronyms`] returns offsets and expansions
//! and never retains the text it read.

use std::fmt;

/// A locale tag, matched case-insensitively.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Locale(String);

impl Locale {
    /// Creates a locale from a tag such as `en` or `pt-BR`.
    pub fn new(tag: &str) -> Self {
        Self(tag.trim().to_lowercase())
    }

    /// The normalised tag.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Locale {
    fn default() -> Self {
        Self::new("en")
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where a dictionary entry came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntrySource {
    /// Shipped with the app.
    BuiltIn,
    /// Contributed by the community and reviewed.
    Community,
    /// Added by the user on this machine.
    User,
}

impl EntrySource {
    /// The stable name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BuiltIn => "built-in",
            Self::Community => "community",
            Self::User => "user",
        }
    }

    /// How authoritative the source is; a higher value wins a conflict.
    pub fn priority(self) -> u8 {
        match self {
            Self::BuiltIn => 0,
            Self::Community => 1,
            Self::User => 2,
        }
    }
}

impl fmt::Display for EntrySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What kind of term an entry explains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JargonKind {
    /// A word or phrase.
    Term,
    /// An acronym, expanded on an uppercase token.
    Acronym,
}

/// One dictionary entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JargonEntry {
    term: String,
    expansion: String,
    locale: Locale,
    source: EntrySource,
    kind: JargonKind,
}

impl JargonEntry {
    /// Creates an entry. Both `locale` and `source` are required.
    pub fn new(
        term: impl Into<String>,
        expansion: impl Into<String>,
        locale: Locale,
        source: EntrySource,
        kind: JargonKind,
    ) -> Self {
        Self {
            term: term.into(),
            expansion: expansion.into(),
            locale,
            source,
            kind,
        }
    }

    /// The term as entered.
    pub fn term(&self) -> &str {
        &self.term
    }

    /// The plain-language expansion.
    pub fn expansion(&self) -> &str {
        &self.expansion
    }

    /// The locale the entry applies to.
    pub fn locale(&self) -> &Locale {
        &self.locale
    }

    /// Where the entry came from.
    pub fn source(&self) -> EntrySource {
        self.source
    }

    /// The kind of term.
    pub fn kind(&self) -> JargonKind {
        self.kind
    }
}

/// An open, versioned jargon dictionary.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JargonDictionary {
    entries: Vec<JargonEntry>,
}

impl JargonDictionary {
    /// Creates an empty dictionary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a dictionary with the shipped entries.
    pub fn with_builtin_defaults() -> Self {
        let mut dictionary = Self::new();
        for entry in builtin_entries() {
            dictionary.add(entry);
        }
        dictionary
    }

    /// How many entries there are.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every entry, in insertion order.
    pub fn entries(&self) -> &[JargonEntry] {
        &self.entries
    }

    /// Adds an entry, replacing any entry with the same locale and term.
    ///
    /// Returns the entry that was replaced, or `None` when this is a new term.
    /// A user entry therefore overrides a built-in of the same term.
    pub fn add(&mut self, entry: JargonEntry) -> Option<JargonEntry> {
        if let Some(slot) = self
            .entries
            .iter_mut()
            .find(|existing| same_term(existing, &entry))
        {
            return Some(std::mem::replace(slot, entry));
        }
        self.entries.push(entry);
        None
    }

    /// Looks a term up in a locale, case-insensitively.
    pub fn lookup(&self, locale: &Locale, term: &str) -> Option<&JargonEntry> {
        self.entries
            .iter()
            .find(|entry| entry.locale == *locale && entry.term.eq_ignore_ascii_case(term))
    }

    /// Whether a term is known in a locale.
    pub fn is_known(&self, locale: &Locale, term: &str) -> bool {
        self.lookup(locale, term).is_some()
    }

    /// Removes the entry for a locale and term, returning it.
    pub fn remove(&mut self, locale: &Locale, term: &str) -> Option<JargonEntry> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.locale == *locale && entry.term.eq_ignore_ascii_case(term))?;
        Some(self.entries.remove(index))
    }

    /// Drops every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Whether two entries describe the same locale and term.
fn same_term(left: &JargonEntry, right: &JargonEntry) -> bool {
    left.locale == right.locale && left.term.eq_ignore_ascii_case(&right.term)
}

/// The shipped entries. A small, obviously safe starter set.
fn builtin_entries() -> Vec<JargonEntry> {
    use JargonKind::{Acronym, Term};
    vec![
        JargonEntry::new(
            "ROI",
            "return on investment: what you gain compared with what you spent",
            Locale::default(),
            EntrySource::BuiltIn,
            Acronym,
        ),
        JargonEntry::new(
            "KPI",
            "key performance indicator: a number used to judge how something is doing",
            Locale::default(),
            EntrySource::BuiltIn,
            Acronym,
        ),
        JargonEntry::new(
            "liability",
            "something you owe, or a legal responsibility",
            Locale::default(),
            EntrySource::BuiltIn,
            Term,
        ),
        JargonEntry::new(
            "ICD",
            "a code doctors use to name a diagnosis",
            Locale::new("en"),
            EntrySource::BuiltIn,
            Acronym,
        ),
    ]
}

/// An acronym found in a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcronymMatch {
    token: String,
    start: usize,
    end: usize,
    expansion: Option<String>,
    source: Option<EntrySource>,
}

impl AcronymMatch {
    /// The token as it appeared.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// The byte offset where the token starts.
    pub fn start(&self) -> usize {
        self.start
    }

    /// The byte offset where the token ends, exclusive.
    pub fn end(&self) -> usize {
        self.end
    }

    /// The expansion, or `None` when the acronym is unknown.
    pub fn expansion(&self) -> Option<&str> {
        self.expansion.as_deref()
    }

    /// Where the expansion came from, or `None` when unknown.
    pub fn source(&self) -> Option<EntrySource> {
        self.source
    }

    /// Whether the acronym is in the dictionary.
    pub fn is_known(&self) -> bool {
        self.expansion.is_some()
    }
}

/// Whether a token is shaped like an acronym.
///
/// Two to eight ASCII alphanumeric characters, at least one letter, and no
/// lowercase letters. `API` and `HTTP2` qualify; `42` and `api` do not.
pub fn is_acronym_token(token: &str) -> bool {
    let length = token.len();
    if !(2..=8).contains(&length) {
        return false;
    }
    let mut letters = 0_usize;
    for ch in token.chars() {
        if !ch.is_ascii_alphanumeric() {
            return false;
        }
        if ch.is_ascii_lowercase() {
            return false;
        }
        if ch.is_ascii_alphabetic() {
            letters += 1;
        }
    }
    letters >= 1
}

/// Expands every acronym in a text, in order of appearance.
///
/// Unknown acronyms are returned with no expansion, so the caller can flag them
/// rather than guess. The function is pure: it retains nothing.
pub fn expand_acronyms(
    text: &str,
    dictionary: &JargonDictionary,
    locale: &Locale,
) -> Vec<AcronymMatch> {
    let mut matches = Vec::new();
    let mut run_start: Option<usize> = None;
    for (index, ch) in text.char_indices() {
        if ch.is_ascii_alphanumeric() {
            if run_start.is_none() {
                run_start = Some(index);
            }
        } else if let Some(start) = run_start.take() {
            if let Some(found) = match_at(text, start, index, dictionary, locale) {
                matches.push(found);
            }
        }
    }
    if let Some(start) = run_start {
        if let Some(found) = match_at(text, start, text.len(), dictionary, locale) {
            matches.push(found);
        }
    }
    matches
}

/// Builds the match for one token span, or `None` when it is not acronym-shaped.
fn match_at(
    text: &str,
    start: usize,
    end: usize,
    dictionary: &JargonDictionary,
    locale: &Locale,
) -> Option<AcronymMatch> {
    let token = &text[start..end];
    if !is_acronym_token(token) {
        return None;
    }
    let (expansion, source) = match dictionary.lookup(locale, token) {
        Some(entry) => (Some(entry.expansion.clone()), Some(entry.source)),
        None => (None, None),
    };
    Some(AcronymMatch {
        token: token.to_string(),
        start,
        end,
        expansion,
        source,
    })
}

/// The unknown acronyms in a text, in order of appearance, without duplicates.
pub fn unknown_acronyms(text: &str, dictionary: &JargonDictionary, locale: &Locale) -> Vec<String> {
    let mut unknown: Vec<String> = Vec::new();
    for found in expand_acronyms(text, dictionary, locale) {
        if !found.is_known() && !unknown.iter().any(|seen| seen == found.token()) {
            unknown.push(found.token().to_string());
        }
    }
    unknown
}

#[cfg(test)]
mod tests;
