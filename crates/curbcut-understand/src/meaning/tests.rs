//! Tests for the meaning-preservation checker.

use super::*;

#[test]
fn reasons_have_names_and_labels() {
    assert_eq!(RejectionReason::MissingEntity.as_str(), "missing-entity");
    assert_eq!(RejectionReason::MissingNumber.as_str(), "missing-number");
    assert_eq!(
        RejectionReason::MissingNegation.as_str(),
        "missing-negation"
    );
    assert_eq!(
        RejectionReason::MissingComparative.as_str(),
        "missing-comparative"
    );
    assert_eq!(RejectionReason::MissingEntity.label(), "named entity");
    assert_eq!(RejectionReason::MissingEntity.plural(), "named entities");
    assert_eq!(RejectionReason::MissingNumber.plural(), "numbers");
    assert_eq!(RejectionReason::ALL.len(), 4);
    assert_eq!(
        RejectionReason::MissingComparative.to_string(),
        "missing-comparative"
    );
}

#[test]
fn category_counts_are_indexed_by_reason() {
    let mut counts = CategoryCounts::default();
    assert!(counts.is_empty());
    assert_eq!(counts.total(), 0);

    counts.set(RejectionReason::MissingEntity, 2);
    counts.set(RejectionReason::MissingNumber, 1);
    assert_eq!(counts.get(RejectionReason::MissingEntity), 2);
    assert_eq!(counts.get(RejectionReason::MissingNumber), 1);
    assert_eq!(counts.get(RejectionReason::MissingNegation), 0);
    assert_eq!(counts.total(), 3);
    assert!(!counts.is_empty());

    assert_eq!(
        counts.reasons(),
        vec![
            RejectionReason::MissingEntity,
            RejectionReason::MissingNumber
        ]
    );
    assert!(CategoryCounts::default().reasons().is_empty());
}

#[test]
fn entities_are_capitalized_runs_trimmed_of_stopwords() {
    assert_eq!(
        extract_entities("Apple Inc. announced record earnings."),
        vec!["apple inc".to_string()]
    );
    assert!(extract_entities("The bank is closed.").is_empty());
    assert!(extract_entities("We cannot refund this.").is_empty());
    assert_eq!(
        extract_entities("Bank of America raised its rate."),
        vec!["bank of america".to_string()]
    );
    assert_eq!(
        extract_entities("The White House said so."),
        vec!["white house".to_string()]
    );
    assert_eq!(
        extract_entities("I saw Google today."),
        vec!["google".to_string()]
    );
    assert!(extract_entities("the quick brown fox").is_empty());
}

#[test]
fn numbers_are_extracted_and_normalized() {
    assert_eq!(
        extract_numbers("It costs $1,200 today."),
        vec!["1200".to_string()]
    );
    assert_eq!(
        extract_numbers("Two values: 3.50 and 4."),
        vec!["3.5".to_string(), "4".to_string()]
    );
    assert_eq!(extract_numbers("A 50% increase."), vec!["50".to_string()]);
    assert_eq!(
        extract_numbers("At 9:30 sharp."),
        vec!["9".to_string(), "30".to_string()]
    );
    assert_eq!(extract_numbers("Minus -5 degrees."), vec!["-5".to_string()]);
    assert!(extract_numbers("No digits here.").is_empty());
}

#[test]
fn number_normalization_unifies_forms() {
    let forms = ["1,200", "1200", "1200.00", "1_200"];
    for form in forms {
        assert_eq!(normalize_number(form), "1200", "{form} did not normalize");
    }
    assert_eq!(normalize_number("0.00"), "0");
    assert_eq!(normalize_number("0.5%"), "0.5");
    assert_eq!(normalize_number("2026."), "2026");
}

#[test]
fn negations_are_extracted_on_word_boundaries() {
    assert_eq!(extract_negations("Do not cancel."), vec!["not".to_string()]);
    assert_eq!(
        extract_negations("We cannot refund."),
        vec!["cannot".to_string()]
    );
    assert_eq!(
        extract_negations("They can't refund."),
        vec!["can't".to_string()]
    );
    assert_eq!(
        extract_negations("No one is here."),
        vec!["no".to_string(), "no one".to_string()]
    );
    assert!(extract_negations("Please note the total.").is_empty());
    assert!(!extract_negations("Nothing is impossible.").is_empty());
}

#[test]
fn comparatives_are_extracted_on_word_boundaries() {
    assert_eq!(
        extract_comparatives("The price is higher than last year."),
        vec!["higher".to_string(), "than".to_string()]
    );
    assert_eq!(
        extract_comparatives("More than half."),
        vec!["more".to_string(), "than".to_string(), "half".to_string()]
    );
    assert!(extract_comparatives("Moreover, it works.").is_empty());
    assert!(!extract_comparatives("The most interesting part.").is_empty());
}

#[test]
fn every_preserved_pair_is_preserved() {
    let checker = MeaningChecker::new();
    for (original, simplified) in preserved_pairs() {
        let decision = checker.check(original, simplified);
        assert!(
            decision.is_preserved(),
            "a valid rewrite was rejected: {decision:?}"
        );
        assert_eq!(checker.recall(original, simplified), 1.0);
    }
}

#[test]
fn every_adversarial_pair_is_rejected() {
    let checker = MeaningChecker::new();
    for (original, simplified) in adversarial_pairs() {
        let decision = checker.check(original, simplified);
        assert!(
            decision.is_rejected(),
            "a meaning-changing rewrite was accepted"
        );
        assert!(checker.recall(original, simplified) < 1.0);
        let report = decision.report().expect("a report");
        assert!(!report.reasons().is_empty());
        assert!(!report.describe().is_empty());
    }
}

#[test]
fn recall_counts_missing_facts() {
    let checker = MeaningChecker::new();
    assert_eq!(
        checker.recall(
            "It costs 5 dollars and 3 cents.",
            "It costs 5 dollars and 3 cents."
        ),
        1.0
    );
    assert_eq!(
        checker.recall("It costs 5 dollars and 3 cents.", "It costs 5 dollars."),
        0.5
    );
    assert_eq!(
        checker.recall("Hello there.", "Hi."),
        1.0,
        "no facts to lose"
    );
}

#[test]
fn repeated_facts_are_counted_as_a_multiset() {
    let original = vec!["5".to_string(), "5".to_string()];
    let candidate = vec!["5".to_string()];
    assert_eq!(missing_count(&original, &candidate), 1);

    let candidate = vec!["5".to_string(), "5".to_string()];
    assert_eq!(missing_count(&original, &candidate), 0);
    assert_eq!(missing_count(&candidate, &original), 0);
}

#[test]
fn a_report_holds_counts_and_no_tokens() {
    let checker = MeaningChecker::new();
    let report = checker
        .report(
            "Bank of America raised its rate by 0.5%.",
            "The bank raised its rate.",
        )
        .expect("a report");

    assert_eq!(report.missing().entities, 1);
    assert_eq!(report.missing().numbers, 1);
    assert_eq!(report.original().entities, 1);
    assert_eq!(report.simplified().entities, 0);
    assert_eq!(report.reasons().len(), 2);
    assert_eq!(report.recall(), 0.0);
    assert_eq!(report.describe(), "missing 1 named entity, 1 number");

    let debug = format!("{report:?}");
    assert!(
        !debug.contains("America"),
        "the report must not carry tokens"
    );
    assert!(!debug.contains("bank"), "the report must not carry tokens");
    assert!(!debug.contains("0.5"));
}

#[test]
fn a_negation_rejection_is_described() {
    let checker = MeaningChecker::new();
    let report = checker
        .report("Do not cancel the policy.", "Cancel the policy.")
        .expect("a report");
    assert_eq!(report.missing().negations, 1);
    assert_eq!(report.describe(), "missing 1 negation");
    assert_eq!(report.reasons(), vec![RejectionReason::MissingNegation]);
}

#[test]
fn a_comparative_rejection_is_described() {
    let checker = MeaningChecker::new();
    let report = checker
        .report(
            "The price is higher than last year.",
            "The price is lower than last year.",
        )
        .expect("a report");
    assert_eq!(report.missing().comparatives, 1);
    assert_eq!(report.describe(), "missing 1 comparative");
}

#[test]
fn a_rejection_log_counts_reasons_only() {
    let checker = MeaningChecker::new();
    let mut log = RejectionLog::default();
    assert!(log.is_empty());
    assert_eq!(log.total(), 0);

    let decision = checker.check(
        "Bank of America raised its rate by 0.5%.",
        "The bank raised its rate.",
    );
    decision.record(&mut log);

    assert_eq!(log.total(), 2);
    assert_eq!(log.count(RejectionReason::MissingEntity), 1);
    assert_eq!(log.count(RejectionReason::MissingNumber), 1);
    assert_eq!(log.count(RejectionReason::MissingNegation), 0);
    assert!(!log.is_empty());

    let counts = log.counts();
    assert_eq!(counts.entities, 1);
    assert_eq!(counts.numbers, 1);

    let debug = format!("{log:?}");
    assert!(!debug.contains("America"));
    assert!(!debug.contains("0.5"));
}

#[test]
fn a_log_merges_and_records_single_reasons() {
    let mut log = RejectionLog::default();
    log.record(RejectionReason::MissingNegation);
    log.record(RejectionReason::MissingNegation);

    let mut other = RejectionLog::default();
    other.record(RejectionReason::MissingEntity);

    log.merge(&other);
    assert_eq!(log.total(), 3);
    assert_eq!(log.count(RejectionReason::MissingNegation), 2);
    assert_eq!(log.count(RejectionReason::MissingEntity), 1);
}

#[test]
fn a_preserved_decision_records_nothing() {
    let checker = MeaningChecker::new();
    let mut log = RejectionLog::default();
    let decision = checker.check("You have 3 messages.", "You have 3 messages.");
    assert!(decision.is_preserved());
    assert!(!decision.is_rejected());
    assert!(decision.report().is_none());
    decision.record(&mut log);
    assert!(log.is_empty());
}

#[test]
fn a_valid_rewrite_is_shown() {
    let checker = MeaningChecker::new();
    let outcome = checker.rewrite(
        "Your deductible is $1,200 per year.",
        "You pay $1,200 each year.",
    );
    assert_eq!(
        outcome,
        MeaningSimplification::Simplified("You pay $1,200 each year.".to_string())
    );
    assert!(!outcome.is_original());
    assert_eq!(outcome.text(), "You pay $1,200 each year.");
}

#[test]
fn a_rejected_rewrite_falls_back_to_the_original() {
    let checker = MeaningChecker::new();
    let original = "Do not cancel the policy.";
    let outcome = checker.rewrite(original, "Cancel the policy.");
    assert_eq!(
        outcome,
        MeaningSimplification::Original(original.to_string())
    );
    assert!(outcome.is_original());
    assert_eq!(outcome.text(), original);
}

#[test]
fn an_empty_candidate_falls_back_to_the_original() {
    let checker = MeaningChecker::new();
    let outcome = checker.rewrite("A plain sentence.", "   ");
    assert_eq!(
        outcome,
        MeaningSimplification::Original("A plain sentence.".to_string())
    );
}

#[test]
fn empty_texts_are_preserved() {
    let checker = MeaningChecker::new();
    assert!(checker.check("", "").is_preserved());
    assert!(checker.check("", "Some text.").is_preserved());
}

#[test]
fn the_checker_is_pure_and_default() {
    let checker = MeaningChecker;
    let first = checker.check("Do not cancel.", "Cancel.");
    let second = checker.check("Do not cancel.", "Cancel.");
    assert_eq!(first, second);
    assert_eq!(checker.recall("Do not cancel.", "Cancel."), 0.0);
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/MEANING.md");
    for word in ["meaning", "entity", "number", "negation", "comparative"] {
        assert!(
            doc.contains(word),
            "docs/MEANING.md does not state `{word}`"
        );
    }
}
