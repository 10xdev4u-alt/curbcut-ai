use super::*;

const CANARY: &str = "CC-CANARY-TEXT-9f86d081884c7d65";

#[test]
fn an_action_is_refused_without_consent() {
    let mut gate = ConsentGate::new();
    let error = gate
        .authorize(ActionClass::Submit, TargetRole::Button)
        .expect_err("no consent was given");
    assert_eq!(error, ConsentError::NotGranted);
    assert!(!gate.is_granted(ActionClass::Submit));
}

#[test]
fn a_granted_prompt_authorizes_the_action_once() {
    let mut gate = ConsentGate::new();
    let prompt = gate.prompt(ActionClass::Submit, TargetRole::Button);
    assert_eq!(prompt.action(), ActionClass::Submit);
    assert_eq!(prompt.target_role(), TargetRole::Button);
    let receipt = gate
        .grant(prompt.token(), ConsentScope::Once)
        .expect("granted");
    assert!(receipt.is_granted());
    assert_eq!(receipt.scope(), Some(ConsentScope::Once));
    assert_eq!(receipt.sequence(), 1);
    assert!(gate.is_granted(ActionClass::Submit));

    let proof = gate
        .authorize(ActionClass::Submit, TargetRole::Button)
        .expect("authorized");
    assert_eq!(proof.action(), ActionClass::Submit);
    assert_eq!(proof.target_role(), TargetRole::Button);
    assert_eq!(proof.sequence(), receipt.sequence());
    assert!(proof.is_valid(&gate));

    assert!(
        gate.authorize(ActionClass::Submit, TargetRole::Button)
            .is_err(),
        "a single-use receipt is consumed"
    );
    assert!(!gate.is_granted(ActionClass::Submit));
}

#[test]
fn a_session_receipt_survives_until_clear() {
    let mut gate = ConsentGate::new();
    let prompt = gate.prompt(ActionClass::Scroll, TargetRole::Generic);
    gate.grant(prompt.token(), ConsentScope::Session)
        .expect("granted");

    for _ in 0..3 {
        assert!(gate
            .authorize(ActionClass::Scroll, TargetRole::Generic)
            .is_ok());
    }
    assert!(gate.is_granted(ActionClass::Scroll));

    let proof = gate
        .authorize(ActionClass::Scroll, TargetRole::Generic)
        .expect("authorized");
    assert!(proof.is_valid(&gate));
    gate.clear();
    assert!(
        !proof.is_valid(&gate),
        "clearing the gate withdraws an outstanding proof"
    );
    assert!(
        gate.authorize(ActionClass::Scroll, TargetRole::Generic)
            .is_err(),
        "clearing the gate withdraws every receipt"
    );
}

#[test]
fn a_denied_prompt_never_authorizes() {
    let mut gate = ConsentGate::new();
    let prompt = gate.prompt(ActionClass::Dismiss, TargetRole::MenuItem);
    let receipt = gate.deny(prompt.token()).expect("denied");
    assert!(!receipt.is_granted());
    assert_eq!(receipt.outcome(), ConsentOutcome::Denied);
    assert_eq!(receipt.scope(), None, "a refusal chooses no scope");
    assert!(!gate.is_granted(ActionClass::Dismiss));
    assert!(gate
        .authorize(ActionClass::Dismiss, TargetRole::MenuItem)
        .is_err());
}

#[test]
fn consent_is_scoped_to_its_action_class() {
    let mut gate = ConsentGate::new();
    let prompt = gate.prompt(ActionClass::Navigate, TargetRole::Link);
    gate.grant(prompt.token(), ConsentScope::Session)
        .expect("granted");

    assert!(gate
        .authorize(ActionClass::Navigate, TargetRole::Link)
        .is_ok());
    assert!(
        gate.authorize(ActionClass::Edit, TargetRole::TextBox)
            .is_err(),
        "consent for one action class does not cover another"
    );
}

#[test]
fn consent_is_scoped_to_its_target_role() {
    let mut gate = ConsentGate::new();
    let prompt = gate.prompt(ActionClass::Submit, TargetRole::Button);
    gate.grant(prompt.token(), ConsentScope::Session)
        .expect("granted");

    assert_eq!(
        gate.authorize(ActionClass::Submit, TargetRole::Link)
            .expect_err("a receipt for one role must not cover another"),
        ConsentError::NotGranted
    );
    let proof = gate
        .authorize(ActionClass::Submit, TargetRole::Button)
        .expect("the matching role authorizes");
    assert_eq!(proof.target_role(), TargetRole::Button);
}

#[test]
fn a_stale_token_is_rejected() {
    let mut gate = ConsentGate::new();
    let prompt = gate.prompt(ActionClass::Invoke, TargetRole::Button);
    let token = prompt.token();
    gate.deny(token).expect("first use");
    assert_eq!(
        gate.grant(token, ConsentScope::Once)
            .expect_err("token reused"),
        ConsentError::UnknownToken
    );
    assert_eq!(
        gate.deny(token).expect_err("token reused"),
        ConsentError::UnknownToken
    );
}

#[test]
fn the_log_records_action_and_role_only() {
    let mut gate = ConsentGate::new();
    let first = gate.prompt(ActionClass::Submit, TargetRole::Button);
    gate.grant(first.token(), ConsentScope::Once)
        .expect("granted");
    let second = gate.prompt(ActionClass::Dismiss, TargetRole::MenuItem);
    gate.deny(second.token()).expect("denied");

    let records: Vec<ConsentRecord> = gate.log().iter().copied().collect();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].sequence, 1);
    assert_eq!(records[0].action, ActionClass::Submit);
    assert_eq!(records[0].target_role, TargetRole::Button);
    assert_eq!(records[0].outcome, ConsentOutcome::Granted);
    assert_eq!(records[0].scope, Some(ConsentScope::Once));
    assert_eq!(records[1].sequence, 2);
    assert_eq!(records[1].action, ActionClass::Dismiss);
    assert_eq!(records[1].target_role, TargetRole::MenuItem);
    assert_eq!(records[1].outcome, ConsentOutcome::Denied);
    assert_eq!(records[1].scope, None);

    let dbg = format!("{:?}", gate.log());
    assert!(dbg.contains("MenuItem"));
    assert!(!dbg.contains(CANARY));
    assert!(!dbg.contains("SECRET"));
}

#[test]
fn the_log_is_bounded() {
    let mut gate = ConsentGate::new();
    for _ in 0..(MAX_RECORDS + 5) {
        let prompt = gate.prompt(ActionClass::Scroll, TargetRole::Generic);
        gate.grant(prompt.token(), ConsentScope::Once)
            .expect("granted");
    }
    assert_eq!(gate.log().len(), MAX_RECORDS);
    assert_eq!(gate.dropped(), 5);
    assert_eq!(gate.log().front().expect("a record").sequence, 6);
}

#[test]
fn pending_prompts_are_bounded_and_evictions_are_counted() {
    let mut gate = ConsentGate::new();
    for _ in 0..(MAX_PENDING_PROMPTS + 5) {
        gate.prompt(ActionClass::Scroll, TargetRole::Generic);
    }
    assert_eq!(gate.pending(), MAX_PENDING_PROMPTS);
    assert_eq!(gate.evicted(), 5);
}

#[test]
fn receipts_are_bounded_and_evictions_are_counted() {
    let mut gate = ConsentGate::new();
    for _ in 0..(MAX_RECEIPTS + 5) {
        let prompt = gate.prompt(ActionClass::Scroll, TargetRole::Generic);
        gate.grant(prompt.token(), ConsentScope::Session)
            .expect("granted");
    }
    assert_eq!(gate.evicted(), 5);
    assert!(gate.is_granted(ActionClass::Scroll));
}

#[test]
fn clear_keeps_the_log_and_drops_pending_prompts() {
    let mut gate = ConsentGate::new();
    let pending = gate.prompt(ActionClass::System, TargetRole::Generic);
    assert_eq!(gate.pending(), 1);
    gate.clear();
    assert_eq!(gate.pending(), 0);
    assert_eq!(
        gate.grant(pending.token(), ConsentScope::Once)
            .expect_err("the prompt is gone"),
        ConsentError::UnknownToken
    );
    assert!(gate.log().is_empty(), "no decision was made");
}

#[test]
fn every_consequence_states_what_will_happen_and_carries_no_content() {
    for action in [
        ActionClass::Navigate,
        ActionClass::Edit,
        ActionClass::Submit,
        ActionClass::Dismiss,
        ActionClass::Scroll,
        ActionClass::Invoke,
        ActionClass::Shortcut,
        ActionClass::System,
    ] {
        let text = action.consequence();
        assert!(text.ends_with('.'), "{action}: {text}");
        assert!(!text.contains(CANARY));
        assert!(!text.contains("SECRET"));
        let prompt = ConsentPrompt::new(ConsentToken::new(1), action, TargetRole::Button);
        let question = prompt.question();
        assert!(question.contains("Allow it?"));
        assert!(question.starts_with(text));
        assert_eq!(format!("{action}"), action.as_str());
    }
    assert_eq!(ActionClass::Submit.as_str(), "submit");
    assert_eq!(TargetRole::MenuItem.as_str(), "menu_item");
    assert_eq!(ConsentScope::Session.as_str(), "session");
    assert_eq!(ConsentOutcome::Denied.as_str(), "denied");
}

#[test]
fn the_document_describes_the_flow() {
    let doc = include_str!("../../../../docs/CONSENT.md");
    assert!(doc.contains("# Action consent flow"));
    assert!(doc.contains("NotGranted"));
    assert!(doc.contains("target role"));
    assert!(doc.contains("is_valid"));
    assert!(!doc.contains(CANARY));
}
