//! Integration tests for the persisted `update_check` root key and the
//! config-built kill switch. Both builds get these tests; the optional
//! transport feature is not needed to dial the update channel.

use std::net::TcpListener;

use curbcut_core::config::Config;
use curbcut_core::net::{update, KillSwitch, NetworkError, NetworkFeature, NetworkPolicy};

fn loopback() -> (TcpListener, std::net::SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback binding");
    let addr = listener.local_addr().expect("a local address");
    (listener, addr)
}

fn config_with(document: &str) -> Config {
    Config::from_toml(document).expect("a valid config document")
}

#[test]
fn absent_update_check_key_denies_the_dial() {
    let config = config_with("version = 1\n");
    assert!(!config.update_check);
    let switch = KillSwitch::from_config(&config);
    assert!(switch.is_engaged());
    assert_eq!(switch.policy(), NetworkPolicy::offline());

    let (listener, addr) = loopback();
    listener
        .set_nonblocking(true)
        .expect("a nonblocking listener");
    assert!(matches!(
        update::dial_from_config(&config, addr),
        Err(NetworkError::Blocked)
    ));
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "a denied dial must not open a socket"
    );
}

#[test]
fn explicit_false_denies_the_dial() {
    let config = config_with("version = 1\nupdate_check = false\n");
    assert!(!config.update_check);
    assert!(KillSwitch::from_config(&config).is_engaged());

    let (_listener, addr) = loopback();
    assert!(matches!(
        update::dial_from_config(&config, addr),
        Err(NetworkError::Blocked)
    ));
}

#[test]
fn true_allows_only_update_check_and_reaches_loopback() {
    let config = config_with("version = 1\nupdate_check = true\n");
    assert!(config.update_check);
    let switch = KillSwitch::from_config(&config);
    assert!(!switch.is_engaged());
    assert_eq!(switch.policy(), NetworkPolicy::updates_only());
    assert!(switch.allows(NetworkFeature::UpdateCheck));
    for feature in NetworkFeature::ALL {
        if feature != NetworkFeature::UpdateCheck {
            assert!(
                !switch.allows(feature),
                "{feature} must stay denied by updates_only"
            );
        }
    }

    let (listener, addr) = loopback();
    update::dial_from_config(&config, addr).expect("an allowed update check reaches loopback");
    listener.accept().expect("a dial reached the listener");

    for feature in NetworkFeature::ALL {
        if feature != NetworkFeature::UpdateCheck {
            assert!(matches!(
                update::dial(&switch, feature, addr),
                Err(NetworkError::Blocked)
            ));
        }
    }
}

#[test]
fn non_boolean_update_check_is_rejected() {
    for document in [
        "update_check = \"yes\"\n",
        "update_check = 1\n",
        "update_check = []\n",
    ] {
        let error = Config::from_toml(document).expect_err("a non-boolean must be rejected");
        assert!(
            error.to_string().contains("update_check"),
            "the error must name the field: {error}"
        );
    }
}

#[test]
fn removing_the_key_revokes_the_choice() {
    let enabled = config_with("version = 1\nupdate_check = true\n");
    let (listener, addr) = loopback();
    update::dial_from_config(&enabled, addr).expect("enabled before removal");
    listener.accept().expect("the first dial reached loopback");

    let without_key = config_with("version = 1\n");
    assert!(!without_key.update_check);
    assert!(matches!(
        update::dial_from_config(&without_key, addr),
        Err(NetworkError::Blocked)
    ));
}
