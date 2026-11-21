use super::*;
use std::net::TcpListener;

/// A loopback listener and its address, so a dial test never leaves the machine.
///
/// The listener must stay alive for the dial to succeed: dropping it closes the
/// port and the connect is refused.
fn loopback() -> (TcpListener, std::net::SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback binding");
    let addr = listener.local_addr().expect("a local address");
    (listener, addr)
}

#[test]
fn every_feature_has_a_stable_unique_name() {
    let mut names: Vec<String> = NetworkFeature::ALL
        .iter()
        .map(|f| f.as_str().to_string())
        .collect();
    let count = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), count, "feature names must be unique");
    assert_eq!(names.len(), 5);
    assert_eq!(NetworkFeature::ALL[0].as_str(), "model_download");
    assert_eq!(NetworkFeature::UpdateCheck.as_str(), "update_check");
    assert_eq!(NetworkFeature::UpdateCheck.to_string(), "update_check");
}

#[test]
fn only_the_update_channel_survives_the_offline_build() {
    for feature in NetworkFeature::ALL {
        assert_eq!(
            feature.survives_offline(),
            feature == NetworkFeature::UpdateCheck,
            "{feature} must report whether the offline build keeps it"
        );
    }
}

#[test]
fn the_mode_matrix_is_most_to_least_restrictive() {
    for feature in NetworkFeature::ALL {
        assert!(
            !NetworkMode::Offline.allows(feature),
            "{feature} is denied offline"
        );
    }
    for feature in NetworkFeature::ALL {
        assert_eq!(
            NetworkMode::UpdatesOnly.allows(feature),
            feature.survives_offline()
        );
    }
    for feature in NetworkFeature::ALL {
        assert!(NetworkMode::Online.allows(feature));
    }
    assert_eq!(
        NetworkMode::ALL.map(|mode| mode.as_str()),
        ["offline", "updates_only", "online"]
    );
}

#[test]
fn the_policy_defaults_to_offline() {
    let policy = NetworkPolicy::default();
    assert_eq!(policy, NetworkPolicy::offline());
    assert_eq!(policy.mode(), NetworkMode::Offline);
    assert!(!policy.allows_any(&NetworkFeature::ALL));
    assert!(!policy.allows_all(&[]));
}

#[test]
fn an_empty_request_is_not_a_request_the_policy_can_grant() {
    let policy = NetworkPolicy::online();
    assert!(!policy.allows_all(&[]));
    assert!(policy.allows_all(&[NetworkFeature::Sync]));
    assert!(policy.allows_any(&[NetworkFeature::Sync, NetworkFeature::Telemetry]));
    assert!(!policy.allows_any(&[]));
}

#[test]
fn a_new_switch_is_engaged_and_denies_everything() {
    let switch = KillSwitch::new(NetworkPolicy::online());
    assert!(switch.is_engaged());
    assert_eq!(switch.policy(), NetworkPolicy::online());
    for feature in NetworkFeature::ALL {
        assert!(!switch.allows(feature), "{feature} is denied while engaged");
    }
}

#[test]
fn releasing_hands_the_decision_to_the_policy() {
    let mut switch = KillSwitch::new(NetworkPolicy::offline());
    switch.release();
    assert!(!switch.is_engaged());
    assert!(!switch.allows(NetworkFeature::ModelDownload));

    switch.set_policy(NetworkPolicy::online());
    assert!(switch.allows(NetworkFeature::ModelDownload));

    // Re-engaging denies everything again, whatever the policy says.
    switch.engage();
    assert!(switch.is_engaged());
    assert!(!switch.allows(NetworkFeature::UpdateCheck));
}

#[test]
fn the_default_switch_denies_everything() {
    let switch = KillSwitch::default();
    assert!(switch.is_engaged());
    assert_eq!(switch.policy(), NetworkPolicy::offline());
    assert!(!switch.allows(NetworkFeature::UpdateCheck));
}

#[test]
fn the_update_channel_is_the_one_path_every_build_keeps() {
    let (_listener, addr) = loopback();
    let mut switch = KillSwitch::new(NetworkPolicy::updates_only());
    switch.release();

    update::dial(&switch, addr).expect("an updates-only policy allows the update check");

    switch.engage();
    assert!(
        matches!(update::dial(&switch, addr), Err(NetworkError::Blocked)),
        "an engaged switch blocks updates too"
    );
}

#[test]
fn an_offline_policy_blocks_the_update_channel() {
    let (_listener, addr) = loopback();
    let mut switch = KillSwitch::new(NetworkPolicy::offline());
    switch.release();
    assert!(matches!(
        update::dial(&switch, addr),
        Err(NetworkError::Blocked)
    ));
}

#[cfg(feature = "net")]
#[test]
fn the_optional_transport_denies_a_held_switch_then_follows_the_policy() {
    let (_listener, addr) = loopback();
    let mut switch = KillSwitch::new(NetworkPolicy::online());

    assert!(
        matches!(
            transport::dial(&switch, NetworkFeature::ModelDownload, addr),
            Err(NetworkError::Blocked)
        ),
        "a fresh switch is engaged"
    );

    switch.release();
    transport::dial(&switch, NetworkFeature::ModelDownload, addr)
        .expect("an online policy allows an optional channel");
    transport::dial(&switch, NetworkFeature::Sync, addr).expect("an online policy allows sync");

    switch.set_policy(NetworkPolicy::offline());
    assert!(
        matches!(
            transport::dial(&switch, NetworkFeature::ModelDownload, addr),
            Err(NetworkError::Blocked)
        ),
        "the policy gate alone stops the channel"
    );
}

#[cfg(feature = "net")]
#[test]
fn the_net_build_has_the_optional_transport() {
    assert!(transport_available());
}

#[cfg(not(feature = "net"))]
#[test]
fn the_offline_build_has_no_optional_transport() {
    assert!(!transport_available());
}

#[test]
fn an_error_names_a_class_and_never_an_address() {
    let blocked = NetworkError::Blocked;
    assert_eq!(blocked.as_str(), "blocked");
    assert_eq!(blocked.to_string(), "blocked");

    let failed = NetworkError::ConnectFailed(std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        "127.0.0.1:9",
    ));
    let rendered = format!("{failed}");
    assert_eq!(rendered, "connect_failed");
    let debugged = format!("{failed:?}");
    assert_eq!(debugged, "connect_failed: connection refused");
    for text in [rendered, debugged] {
        assert!(
            !text.contains("127.0.0.1"),
            "an error must not carry the address it was given: {text}"
        );
    }
}

#[test]
fn the_document_describes_the_two_gates() {
    let doc = include_str!("../../../../docs/NETWORK_POLICY.md");
    assert!(doc.contains("# Network policy"));
    assert!(doc.contains("kill-switch"));
    assert!(doc.contains("offline"));
    assert!(doc.contains("UpdateCheck"));
    assert!(!doc.contains(CANARY));
}

const CANARY: &str = "CC-CANARY-TEXT-9f86d081884c7d65";
