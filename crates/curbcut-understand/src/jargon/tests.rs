//! Tests for the jargon dictionary and acronym expansion.

use super::*;

fn english() -> Locale {
    Locale::default()
}

#[test]
fn locales_are_normalised() {
    assert_eq!(Locale::new("EN").as_str(), "en");
    assert_eq!(Locale::new("  pt-BR  ").as_str(), "pt-br");
    assert_eq!(Locale::default().as_str(), "en");
    assert_eq!(Locale::new("EN"), Locale::new("en"));
}

#[test]
fn sources_have_a_priority() {
    assert_eq!(EntrySource::User.priority(), 2);
    assert!(EntrySource::User.priority() > EntrySource::Community.priority());
    assert!(EntrySource::Community.priority() > EntrySource::BuiltIn.priority());
    assert_eq!(EntrySource::BuiltIn.as_str(), "built-in");
}

#[test]
fn an_entry_carries_a_locale_and_a_source() {
    let entry = JargonEntry::new(
        "API",
        "application programming interface",
        Locale::new("en"),
        EntrySource::BuiltIn,
        JargonKind::Acronym,
    );

    assert_eq!(entry.term(), "API");
    assert_eq!(entry.expansion(), "application programming interface");
    assert_eq!(entry.locale(), &Locale::new("en"));
    assert_eq!(entry.source(), EntrySource::BuiltIn);
    assert_eq!(entry.kind(), JargonKind::Acronym);
}

#[test]
fn the_builtin_dictionary_has_entries() {
    let dictionary = JargonDictionary::with_builtin_defaults();

    assert!(dictionary.len() >= 4);
    assert!(dictionary.is_known(&english(), "ROI"));
    assert!(
        dictionary.is_known(&english(), "roi"),
        "lookup is case-insensitive"
    );
    assert!(!dictionary.is_known(&english(), "ZZZ"));
    for entry in dictionary.entries() {
        assert_eq!(entry.source(), EntrySource::BuiltIn);
        assert_eq!(entry.locale(), &english());
    }
}

#[test]
fn adding_a_term_keeps_it() {
    let mut dictionary = JargonDictionary::new();
    assert!(dictionary.is_empty());

    dictionary.add(JargonEntry::new(
        "deductible",
        "the amount you pay before insurance starts to pay",
        english(),
        EntrySource::User,
        JargonKind::Term,
    ));

    assert_eq!(dictionary.len(), 1);
    assert!(dictionary.is_known(&english(), "deductible"));
    assert_eq!(
        dictionary
            .lookup(&english(), "DEDUCTIBLE")
            .map(JargonEntry::expansion),
        Some("the amount you pay before insurance starts to pay")
    );
}

#[test]
fn a_user_entry_overrides_a_builtin() {
    let mut dictionary = JargonDictionary::with_builtin_defaults();

    let replaced = dictionary.add(JargonEntry::new(
        "ROI",
        "a user's own meaning",
        english(),
        EntrySource::User,
        JargonKind::Acronym,
    ));

    let replaced = replaced.expect("the built-in was replaced");
    assert_eq!(replaced.source(), EntrySource::BuiltIn);
    assert_eq!(dictionary.len(), 4, "an override replaces, it does not add");
    assert_eq!(
        dictionary
            .lookup(&english(), "ROI")
            .map(JargonEntry::expansion),
        Some("a user's own meaning")
    );
    assert_eq!(
        dictionary
            .lookup(&english(), "ROI")
            .map(JargonEntry::source),
        Some(EntrySource::User)
    );
}

#[test]
fn locales_do_not_collide() {
    let mut dictionary = JargonDictionary::new();
    dictionary.add(JargonEntry::new(
        "nota",
        "a note",
        Locale::new("en"),
        EntrySource::BuiltIn,
        JargonKind::Term,
    ));

    // The same term in another locale is a different entry.
    assert!(dictionary
        .add(JargonEntry::new(
            "nota",
            "a grade",
            Locale::new("pt-br"),
            EntrySource::BuiltIn,
            JargonKind::Term,
        ))
        .is_none());
    assert_eq!(dictionary.len(), 2);
    assert!(dictionary.is_known(&Locale::new("en"), "nota"));
    assert!(dictionary.is_known(&Locale::new("pt-br"), "nota"));
}

#[test]
fn remove_takes_the_entry_out() {
    let mut dictionary = JargonDictionary::with_builtin_defaults();

    let removed = dictionary.remove(&english(), "KPI");
    assert_eq!(
        removed.map(|entry| entry.term().to_string()),
        Some("KPI".to_string())
    );
    assert!(!dictionary.is_known(&english(), "KPI"));
    assert_eq!(dictionary.remove(&english(), "KPI"), None);
}

#[test]
fn clear_empties_the_dictionary() {
    let mut dictionary = JargonDictionary::with_builtin_defaults();
    dictionary.clear();
    assert!(dictionary.is_empty());
    assert_eq!(dictionary.entries(), &[]);
}

#[test]
fn acronym_tokens_are_recognised_by_shape() {
    assert!(is_acronym_token("API"));
    assert!(is_acronym_token("ROI"));
    assert!(is_acronym_token("HTTP2"));
    assert!(is_acronym_token("A1"));

    assert!(
        !is_acronym_token("A"),
        "one character is a word, not an acronym"
    );
    assert!(!is_acronym_token("42"), "no letters");
    assert!(!is_acronym_token("api"), "lowercase");
    assert!(!is_acronym_token("APIs"), "trailing lowercase");
    assert!(!is_acronym_token("ABCDEFGHI"), "too long");
    assert!(!is_acronym_token(""));
}

#[test]
fn known_acronyms_are_expanded_with_offsets() {
    let dictionary = JargonDictionary::with_builtin_defaults();

    let matches = expand_acronyms("Your ROI and KPI.", &dictionary, &english());

    assert_eq!(matches.len(), 2);
    let roi = &matches[0];
    assert_eq!(roi.token(), "ROI");
    assert_eq!(roi.start(), 5);
    assert_eq!(roi.end(), 8);
    assert!(roi.is_known());
    assert_eq!(
        roi.expansion(),
        Some("return on investment: what you gain compared with what you spent")
    );
    assert_eq!(roi.source(), Some(EntrySource::BuiltIn));

    let kpi = &matches[1];
    assert_eq!(kpi.token(), "KPI");
    assert_eq!(kpi.start(), 13);
    assert_eq!(kpi.end(), 16);
}

#[test]
fn unknown_acronyms_are_flagged_not_guessed() {
    let dictionary = JargonDictionary::with_builtin_defaults();

    let matches = expand_acronyms("Your EBITDA grew.", &dictionary, &english());

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].token(), "EBITDA");
    assert!(!matches[0].is_known());
    assert_eq!(matches[0].expansion(), None);
    assert_eq!(matches[0].source(), None);
}

#[test]
fn ordinary_words_are_not_acronyms() {
    let dictionary = JargonDictionary::with_builtin_defaults();

    // Sentence-case words and single letters are not acronym-shaped.
    let matches = expand_acronyms("The API is a tool. I use it.", &dictionary, &english());

    let tokens: Vec<&str> = matches.iter().map(AcronymMatch::token).collect();
    assert_eq!(tokens, vec!["API"]);
}

#[test]
fn unknown_acronyms_are_listed_once() {
    let dictionary = JargonDictionary::with_builtin_defaults();

    let unknown = unknown_acronyms("EBITDA then EBITDA again.", &dictionary, &english());

    assert_eq!(unknown, vec!["EBITDA".to_string()]);
}

#[test]
fn acronyms_in_a_run_are_split_on_non_alphanumerics() {
    let dictionary = JargonDictionary::with_builtin_defaults();

    let matches = expand_acronyms("(ROI, KPI)", &dictionary, &english());

    let tokens: Vec<&str> = matches.iter().map(AcronymMatch::token).collect();
    assert_eq!(tokens, vec!["ROI", "KPI"]);
}

#[test]
fn expansion_is_pure() {
    let dictionary = JargonDictionary::with_builtin_defaults();
    let text = "ROI";

    let first = expand_acronyms(text, &dictionary, &english());
    let second = expand_acronyms(text, &dictionary, &english());

    assert_eq!(first, second);
    assert_eq!(
        dictionary.len(),
        4,
        "expanding does not mutate the dictionary"
    );
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/JARGON.md");
    for word in ["jargon", "acronym", "locale", "source", "unknown"] {
        assert!(doc.contains(word), "docs/JARGON.md does not state `{word}`");
    }
}
