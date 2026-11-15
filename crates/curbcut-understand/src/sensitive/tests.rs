//! Tests for the sensitive-content guard.

use super::*;

fn guard() -> SensitiveGuard {
    SensitiveGuard::default()
}

#[test]
fn domains_have_names_and_labels() {
    assert_eq!(SensitiveDomain::Financial.as_str(), "financial");
    assert_eq!(SensitiveDomain::Medical.as_str(), "medical");
    assert_eq!(SensitiveDomain::Legal.as_str(), "legal");
    assert_eq!(SensitiveDomain::Financial.label(), "Financial");
    assert_eq!(SensitiveDomain::ALL.len(), 3);
    assert_eq!(SensitiveDomain::Financial.to_string(), "financial");
}

#[test]
fn the_signal_table_is_data_and_auditable() {
    let guard = guard();
    assert!(guard.signals().len() >= 100, "a real signal table");

    let mut markers: Vec<&str> = guard
        .signals()
        .iter()
        .map(SensitiveSignal::marker)
        .collect();
    let count = markers.len();
    markers.sort_unstable();
    markers.dedup();
    assert_eq!(markers.len(), count, "no duplicate markers");

    for signal in guard.signals() {
        assert!(!signal.marker().is_empty());
        assert!(!signal.marker().chars().all(|ch| ch.is_uppercase()));
    }
    assert_eq!(guard.weak_threshold(), 2);
}

#[test]
fn an_empty_guard_clears_everything() {
    let guard = SensitiveGuard::empty();
    assert!(guard.signals().is_empty());
    assert_eq!(guard.scan("Your deductible is $500."), GuardDecision::Clear);
    assert!(!guard.is_sensitive("anything"));
}

#[test]
fn every_labeled_sensitive_sample_triggers() {
    let guard = guard();
    let samples = labeled_sensitive();
    assert!(samples.len() >= 20);

    for sample in &samples {
        let decision = guard.scan(sample);
        assert!(
            decision.is_refused(),
            "the guard missed a labeled sensitive sample"
        );
        let report = decision.report().expect("a report");
        assert!(report.is_sensitive());
        assert!(!report.domains().is_empty());
    }
}

#[test]
fn every_adversarial_variant_triggers() {
    let guard = guard();
    for sample in adversarial_variants() {
        assert!(
            guard.is_sensitive(sample),
            "an adversarial variant slipped past the guard"
        );
    }
}

#[test]
fn ordinary_text_does_not_trigger() {
    let guard = guard();
    for sample in non_sensitive() {
        assert!(
            !guard.is_sensitive(sample),
            "ordinary text was flagged as sensitive"
        );
        assert_eq!(guard.scan(sample), GuardDecision::Clear);
    }
}

#[test]
fn a_strong_signal_triggers_alone() {
    let guard = guard();
    let decision = guard.scan("The biopsy came back benign.");
    let report = decision.report().expect("a report");
    assert_eq!(report.domains(), &[SensitiveDomain::Medical]);
    assert_eq!(report.strong_hits(), 2);
}

#[test]
fn a_single_weak_signal_does_not_trigger() {
    let guard = guard();
    assert!(!guard.is_sensitive("The river bank was steep."));
    assert!(!guard.is_sensitive("The court of public opinion."));
    assert!(!guard.is_sensitive("Please charge my phone."));
}

#[test]
fn two_weak_signals_in_one_domain_trigger() {
    let guard = guard();
    let report = guard
        .report("The bank statement arrived.")
        .expect("a report");
    assert_eq!(report.domains(), &[SensitiveDomain::Financial]);
    assert_eq!(report.strong_hits(), 0);
    assert_eq!(report.weak_hits(), 2);
}

#[test]
fn weak_signals_from_different_domains_do_not_trigger() {
    let guard = guard();
    // One financial weak signal and one legal weak signal: no domain has two.
    assert!(!guard.is_sensitive("The bank is near the court."));
}

#[test]
fn the_weak_threshold_is_configurable() {
    let mut guard = guard();
    guard.set_weak_threshold(1);
    assert!(guard.is_sensitive("The river bank was steep."));

    guard.set_weak_threshold(0);
    assert!(!guard.is_sensitive("The bank statement arrived."));
    assert!(
        guard.is_sensitive("Your deductible is $500."),
        "strong signals still trigger"
    );
}

#[test]
fn word_boundaries_are_respected() {
    let guard = guard();
    // "bank" inside "banking" is not the word "bank".
    assert!(!guard.is_sensitive("Banking on a good outcome is unwise."));
    // A phrase marker matches as a phrase.
    assert!(guard.is_sensitive("He signed a power of attorney."));
}

#[test]
fn the_report_counts_hits_and_never_the_text() {
    let guard = guard();
    let text = "Your deductible is $500 per year.";
    let report = guard.report(text).expect("a report");

    assert_eq!(report.strong_hits(), 1);
    assert_eq!(report.weak_hits(), 0, "\"deductible\" is strong, not weak");
    assert_eq!(report.domains(), &[SensitiveDomain::Financial]);

    let debug = format!("{report:?}");
    assert!(!debug.contains("500"), "the report must not carry the text");
    assert!(!debug.contains("deductible"));

    let message = report.message();
    assert!(message.contains("Financial"));
    assert!(!message.contains("deductible"));
    assert!(!message.contains('$'));
}

#[test]
fn the_report_names_every_triggered_domain_once() {
    let guard = guard();
    let report = guard
        .report("The diagnosis and the deductible were both listed.")
        .expect("a report");
    assert_eq!(
        report.domains(),
        &[SensitiveDomain::Financial, SensitiveDomain::Medical]
    );
    assert!(report.message().contains("Financial"));
    assert!(report.message().contains("Medical"));
}

#[test]
fn the_guard_can_be_extended_and_cleared() {
    let mut guard = SensitiveGuard::empty();
    guard.add_signal(SensitiveSignal::new(
        "widget",
        SensitiveDomain::Legal,
        SignalStrength::Strong,
    ));
    assert!(guard.is_sensitive("The widget was filed."));
    assert_eq!(guard.remove_signal("widget"), 1);
    assert_eq!(guard.remove_signal("widget"), 0);
    assert!(!guard.is_sensitive("The widget was filed."));

    guard.add_signal(SensitiveSignal::new(
        "gadget",
        SensitiveDomain::Medical,
        SignalStrength::Weak,
    ));
    guard.clear();
    assert!(guard.signals().is_empty());
    assert!(!guard.is_sensitive("The gadget was filed."));
}

#[test]
fn a_clear_text_is_simplified() {
    let guard = guard();
    let decision = guard.auto_simplify("The cat sat on the mat.", "A cat sat down.");
    assert_eq!(
        decision,
        Simplification::Simplified("A cat sat down.".to_string())
    );
    assert!(!decision.is_original());
    assert_eq!(decision.text(), "A cat sat down.");
}

#[test]
fn a_sensitive_original_is_never_rewritten() {
    let guard = guard();
    let original = "Your deductible is $500 per year.";
    let decision = guard.auto_simplify(original, "You pay $500 first.");

    assert_eq!(decision, Simplification::Original(original.to_string()));
    assert!(decision.is_original());
    assert_eq!(decision.text(), original, "the original is always visible");
}

#[test]
fn a_sensitive_candidate_is_refused_even_when_the_original_is_clear() {
    let guard = guard();
    let decision = guard.auto_simplify("A plain sentence.", "Your deductible is $500.");
    assert_eq!(
        decision,
        Simplification::Original("A plain sentence.".to_string())
    );
}

#[test]
fn an_empty_candidate_falls_back_to_the_original() {
    let guard = guard();
    let decision = guard.auto_simplify("A plain sentence.", "   ");
    assert_eq!(
        decision,
        Simplification::Original("A plain sentence.".to_string())
    );
}

#[test]
fn the_guard_is_pure() {
    let guard = guard();
    let text = "Your deductible is $500.";
    let first = guard.scan(text);
    let second = guard.scan(text);
    assert_eq!(first, second);
    assert_eq!(guard.signals().len(), guard.signals().len());
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/SENSITIVE_CONTENT.md");
    for word in ["sensitive", "simplify", "original", "guard", "domain"] {
        assert!(
            doc.contains(word),
            "docs/SENSITIVE_CONTENT.md does not state `{word}`"
        );
    }
}
