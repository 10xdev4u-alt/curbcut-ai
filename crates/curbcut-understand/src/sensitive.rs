//! Sensitive-content guard.
//!
//! Some text must never be auto-simplified. A financial, medical, or legal
//! sentence can change meaning in a way a reader cannot see, so the guard
//! refuses to rewrite it and the original stays visible. See
//! [`../../../docs/SENSITIVE_CONTENT.md`](../../../docs/SENSITIVE_CONTENT.md).
//!
//! Four rules hold:
//!
//! - **The guard is conservative.** A strong signal triggers on its own; two
//!   weak signals trigger only when they agree on a domain. Ordinary prose with
//!   one ambiguous word does not trigger.
//! - **The original is always visible.** [`SensitiveGuard::auto_simplify`]
//!   returns [`Simplification::Original`] with the original text when the guard
//!   refuses, and never returns a rewrite.
//! - **Nothing is logged.** [`SensitiveReport`] counts hits and names domains;
//!   it never carries the text it scanned.
//! - **The signal table is data.** [`SensitiveGuard::signals`] is the auditable
//!   list, and it can be extended or cleared without touching the logic.

use std::fmt;

/// A domain whose content is sensitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SensitiveDomain {
    /// Money, banking, credit, tax.
    Financial,
    /// Health, diagnosis, medication.
    Medical,
    /// Law, courts, contracts, rights.
    Legal,
}

impl SensitiveDomain {
    /// Every supported domain, in report order.
    pub const ALL: [SensitiveDomain; 3] = [Self::Financial, Self::Medical, Self::Legal];

    /// The stable name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Financial => "financial",
            Self::Medical => "medical",
            Self::Legal => "legal",
        }
    }

    /// The human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Financial => "Financial",
            Self::Medical => "Medical",
            Self::Legal => "Legal",
        }
    }
}

impl fmt::Display for SensitiveDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How strong a signal is on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalStrength {
    /// Triggers the guard by itself.
    Strong,
    /// Needs a second weak signal from the same domain.
    Weak,
}

/// One marker in the signal table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SensitiveSignal {
    marker: &'static str,
    domain: SensitiveDomain,
    strength: SignalStrength,
}

impl SensitiveSignal {
    /// Creates a signal.
    pub const fn new(
        marker: &'static str,
        domain: SensitiveDomain,
        strength: SignalStrength,
    ) -> Self {
        Self {
            marker,
            domain,
            strength,
        }
    }

    /// The text to look for.
    pub fn marker(&self) -> &'static str {
        self.marker
    }

    /// The domain it belongs to.
    pub fn domain(&self) -> SensitiveDomain {
        self.domain
    }

    /// The strength.
    pub fn strength(&self) -> SignalStrength {
        self.strength
    }
}

/// What the guard found, without the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensitiveReport {
    domains: Vec<SensitiveDomain>,
    strong_hits: usize,
    weak_hits: usize,
}

impl SensitiveReport {
    /// The domains that triggered, in a stable order.
    pub fn domains(&self) -> &[SensitiveDomain] {
        &self.domains
    }

    /// Whether the guard refused.
    pub fn is_sensitive(&self) -> bool {
        !self.domains.is_empty()
    }

    /// How many strong signals matched.
    pub fn strong_hits(&self) -> usize {
        self.strong_hits
    }

    /// How many weak signals matched, across all domains.
    pub fn weak_hits(&self) -> usize {
        self.weak_hits
    }

    /// A warning the UI can show. It names domains, never the text.
    pub fn message(&self) -> String {
        let names: Vec<&str> = self.domains.iter().map(|domain| domain.label()).collect();
        format!(
            "This looks like {} content. It was left as written.",
            names.join(", ")
        )
    }
}

/// The result of scanning a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardDecision {
    /// Nothing sensitive was found.
    Clear,
    /// The text must not be auto-simplified.
    Refused(SensitiveReport),
}

impl GuardDecision {
    /// Whether the guard refused.
    pub fn is_refused(&self) -> bool {
        matches!(self, Self::Refused(_))
    }

    /// The report, when the guard refused.
    pub fn report(&self) -> Option<&SensitiveReport> {
        match self {
            Self::Clear => None,
            Self::Refused(report) => Some(report),
        }
    }
}

/// What a caller may show after simplification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Simplification {
    /// The rewrite is safe to show.
    Simplified(String),
    /// The guard refused; show the original.
    Original(String),
}

impl Simplification {
    /// The text to show.
    pub fn text(&self) -> &str {
        match self {
            Self::Simplified(text) | Self::Original(text) => text,
        }
    }

    /// Whether this is the untouched original.
    pub fn is_original(&self) -> bool {
        matches!(self, Self::Original(_))
    }
}

/// The sensitive-content guard.
#[derive(Debug, Clone)]
pub struct SensitiveGuard {
    signals: Vec<SensitiveSignal>,
    weak_threshold: usize,
}

impl Default for SensitiveGuard {
    fn default() -> Self {
        Self::new(2)
    }
}

impl SensitiveGuard {
    /// Creates a guard with the built-in signal table.
    pub fn new(weak_threshold: usize) -> Self {
        Self {
            signals: builtin_signals(),
            weak_threshold,
        }
    }

    /// Creates a guard with no signals.
    pub fn empty() -> Self {
        Self {
            signals: Vec::new(),
            weak_threshold: 2,
        }
    }

    /// The auditable signal table.
    pub fn signals(&self) -> &[SensitiveSignal] {
        &self.signals
    }

    /// How many same-domain weak signals are needed to trigger.
    pub fn weak_threshold(&self) -> usize {
        self.weak_threshold
    }

    /// Sets the weak threshold. Zero disables weak signals.
    pub fn set_weak_threshold(&mut self, weak_threshold: usize) {
        self.weak_threshold = weak_threshold;
    }

    /// Adds a signal.
    pub fn add_signal(&mut self, signal: SensitiveSignal) {
        self.signals.push(signal);
    }

    /// Removes every signal whose marker matches, returning how many.
    pub fn remove_signal(&mut self, marker: &str) -> usize {
        let before = self.signals.len();
        self.signals.retain(|signal| signal.marker != marker);
        before - self.signals.len()
    }

    /// Drops every signal.
    pub fn clear(&mut self) {
        self.signals.clear();
    }

    /// Scans a text.
    pub fn scan(&self, text: &str) -> GuardDecision {
        let lower = text.to_lowercase();
        let mut strong_by_domain: [usize; 3] = [0; 3];
        let mut weak_by_domain: [usize; 3] = [0; 3];
        let mut strong_hits = 0_usize;
        let mut weak_hits = 0_usize;

        for signal in &self.signals {
            if !contains_signal(&lower, signal.marker) {
                continue;
            }
            let index = domain_index(signal.domain);
            match signal.strength {
                SignalStrength::Strong => {
                    strong_hits += 1;
                    strong_by_domain[index] += 1;
                }
                SignalStrength::Weak => {
                    weak_hits += 1;
                    weak_by_domain[index] += 1;
                }
            }
        }

        let threshold = self.weak_threshold;
        let domains: Vec<SensitiveDomain> = SensitiveDomain::ALL
            .into_iter()
            .enumerate()
            .filter(|(index, _)| {
                strong_by_domain[*index] > 0
                    || (threshold > 0 && weak_by_domain[*index] >= threshold)
            })
            .map(|(_, domain)| domain)
            .collect();

        if domains.is_empty() {
            return GuardDecision::Clear;
        }
        GuardDecision::Refused(SensitiveReport {
            domains,
            strong_hits,
            weak_hits,
        })
    }

    /// The report, or `None` when the text is clear.
    pub fn report(&self, text: &str) -> Option<SensitiveReport> {
        match self.scan(text) {
            GuardDecision::Clear => None,
            GuardDecision::Refused(report) => Some(report),
        }
    }

    /// The domains that triggered, in a stable order.
    pub fn domains(&self, text: &str) -> Vec<SensitiveDomain> {
        self.report(text)
            .map(|report| report.domains.to_vec())
            .unwrap_or_default()
    }

    /// Whether the text must not be auto-simplified.
    pub fn is_sensitive(&self, text: &str) -> bool {
        self.report(text).is_some()
    }

    /// Decides what to show after a rewrite.
    ///
    /// The original is returned whenever either the original or the candidate
    /// is sensitive, or when the candidate is empty. A caller can therefore
    /// never show a rewrite the guard has not cleared.
    pub fn auto_simplify(&self, original: &str, candidate: &str) -> Simplification {
        if self.is_sensitive(original)
            || self.is_sensitive(candidate)
            || candidate.trim().is_empty()
        {
            return Simplification::Original(original.to_string());
        }
        Simplification::Simplified(candidate.to_string())
    }
}

/// The index of a domain in [`SensitiveDomain::ALL`].
fn domain_index(domain: SensitiveDomain) -> usize {
    match domain {
        SensitiveDomain::Financial => 0,
        SensitiveDomain::Medical => 1,
        SensitiveDomain::Legal => 2,
    }
}

/// Whether a lowercased text contains a marker.
///
/// A single-word marker must sit on word boundaries, so `bank` matches
/// "river bank" but not "banking". A multi-word marker is a phrase and is
/// matched as a substring.
fn contains_signal(lower: &str, marker: &str) -> bool {
    if marker.contains(' ') {
        return lower.contains(marker);
    }
    let bytes = lower.as_bytes();
    lower.match_indices(marker).any(|(at, _)| {
        let before_ok = at == 0 || !is_word_byte(bytes[at - 1]);
        let end = at + marker.len();
        let after_ok = end == bytes.len() || !is_word_byte(bytes[end]);
        before_ok && after_ok
    })
}

/// Whether a byte continues a word.
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The shipped signal table.
///
/// Strong markers are unambiguous domain words and phrases. Weak markers are
/// ambiguous single words that only trigger in a pair from the same domain.
fn builtin_signals() -> Vec<SensitiveSignal> {
    use SignalStrength::{Strong, Weak};

    let financial_strong = [
        "deductible",
        "co-pay",
        "copay",
        "out-of-pocket",
        "credit score",
        "bank account",
        "routing number",
        "account number",
        "wire transfer",
        "interest rate",
        "annual percentage rate",
        "mortgage",
        "loan",
        "invoice",
        "balance due",
        "payment due",
        "social security",
        "credit card",
        "debit card",
        "tax",
        "bankruptcy",
        "collateral",
        "lien",
    ];
    let medical_strong = [
        "diagnosis",
        "diagnosed",
        "prognosis",
        "prescription",
        "dosage",
        "symptom",
        "medication",
        "pharmacy",
        "biopsy",
        "malignant",
        "benign",
        "oncology",
        "chemotherapy",
        "blood pressure",
        "heart rate",
        "allergic",
        "allergy",
        "side effect",
        "overdose",
        "seizure",
        "stroke",
        "pathology",
        "lab results",
        "pre-existing",
        "medical record",
        "hipaa",
    ];
    let legal_strong = [
        "plaintiff",
        "defendant",
        "subpoena",
        "statute",
        "ordinance",
        "lawsuit",
        "litigation",
        "affidavit",
        "deposition",
        "verdict",
        "probation",
        "parole",
        "warrant",
        "indictment",
        "jurisdiction",
        "liable",
        "liability",
        "tort",
        "breach of contract",
        "terms of service",
        "power of attorney",
        "custody",
        "divorce",
        "eviction",
        "lease agreement",
        "attorney",
        "lawyer",
    ];

    let financial_weak = [
        "bank",
        "account",
        "charge",
        "bill",
        "interest",
        "balance",
        "statement",
        "fee",
        "fine",
        "penalty",
        "payment",
        "due",
        "credit",
        "debit",
        "transfer",
        "deposit",
        "withdrawal",
        "premium",
        "claim",
    ];
    let medical_weak = [
        "coverage", "benefit", "provider", "patient", "doctor", "hospital", "clinic", "ward",
        "nurse", "dose", "drug", "pill", "health",
    ];
    let legal_weak = [
        "policy",
        "court",
        "state",
        "suit",
        "case",
        "party",
        "parties",
        "agreement",
        "contract",
        "terms",
        "rights",
        "violation",
        "offense",
        "crime",
        "police",
        "legal",
    ];

    let mut signals = Vec::new();
    for marker in financial_strong {
        signals.push(SensitiveSignal::new(
            marker,
            SensitiveDomain::Financial,
            Strong,
        ));
    }
    for marker in medical_strong {
        signals.push(SensitiveSignal::new(
            marker,
            SensitiveDomain::Medical,
            Strong,
        ));
    }
    for marker in legal_strong {
        signals.push(SensitiveSignal::new(marker, SensitiveDomain::Legal, Strong));
    }
    for marker in financial_weak {
        signals.push(SensitiveSignal::new(
            marker,
            SensitiveDomain::Financial,
            Weak,
        ));
    }
    for marker in medical_weak {
        signals.push(SensitiveSignal::new(marker, SensitiveDomain::Medical, Weak));
    }
    for marker in legal_weak {
        signals.push(SensitiveSignal::new(marker, SensitiveDomain::Legal, Weak));
    }
    signals
}

/// A labeled sensitive set: every entry must trigger the guard.
pub fn labeled_sensitive() -> Vec<&'static str> {
    vec![
        "Your deductible is $500 per year.",
        "The credit card APR is 21.99%.",
        "Your bank account routing number is on file.",
        "A wire transfer of $12,000 was initiated.",
        "Your mortgage payment is due on the first.",
        "The invoice shows a balance due of $340.",
        "Your credit score changed this month.",
        "The diagnosis was hypertension.",
        "Take this prescription twice daily.",
        "Your blood pressure reading was 140 over 90.",
        "The biopsy came back benign.",
        "You reported nausea as a side effect.",
        "The oncology team will see you Friday.",
        "Your lab results are ready.",
        "The plaintiff filed a motion to dismiss.",
        "A subpoena was served on Monday.",
        "The statute of limitations expires next year.",
        "The defendant pleaded not guilty.",
        "This is governed by the terms of service.",
        "The court issued a warrant.",
        "You have the right to an attorney.",
    ]
}

/// Adversarial variants: casing, spacing, punctuation, and hyphenation.
pub fn adversarial_variants() -> Vec<&'static str> {
    vec![
        "YOUR DEDUCTIBLE IS $500",
        "your  deductible   is 500",
        "deductible: $500.",
        "The  co-pay  amount is twenty dollars.",
        "DIAGNOSIS: HYPERTENSION",
        "pre-existing condition",
        "power of attorney",
        "PLAINTIFF alleges breach of contract.",
        "The out-of-pocket maximum is $8,000.",
    ]
}

/// Ordinary text that must not trigger the guard.
pub fn non_sensitive() -> Vec<&'static str> {
    vec![
        "The river bank was steep.",
        "The state flag flew over the capitol.",
        "Please charge my phone.",
        "The bill for the lamp was fifteen dollars.",
        "A good policy is to double-check your work.",
        "The account of the battle was gripping.",
        "Interest in the subject grew.",
        "The court of public opinion.",
        "Add two eggs and stir.",
        "The weather is mild today.",
        "She walked to the park.",
        "Banking on a good outcome is unwise.",
    ]
}

#[cfg(test)]
mod tests;
