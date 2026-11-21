//! Offline network probe.
//!
//! A smoke run of the gate for the socket guard. It binds a loopback listener,
//! exercises every channel against a fresh engaged switch and against a
//! released offline policy, and exits 0 only when every dial that was not
//! allowed returned [`NetworkError::Blocked`].
//!
//! Every dial here is refused, so the probe opens no socket:
//! `scripts/socket_guard.py` runs it and asserts zero `connect(2)` calls. If a
//! gate regressed, the dial would reach the listener and this probe would
//! panic, so the guard and the probe fail together. See
//! `docs/NETWORK_POLICY.md`.

use std::net::{TcpListener, TcpStream};

use curbcut_core::net::{
    transport_available, update, KillSwitch, NetworkError, NetworkFeature, NetworkPolicy,
};

/// A loopback listener that stays alive for the whole run.
///
/// Binding it proves the address is reachable, so a `Blocked` result is a
/// policy decision and not a dead port.
fn listener() -> (TcpListener, std::net::SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback listener");
    let addr = listener.local_addr().expect("a loopback address");
    (listener, addr)
}

fn main() {
    let (listener, addr) = listener();
    println!("transport_available: {}", transport_available());

    let engaged = KillSwitch::new(NetworkPolicy::updates_only());
    report("engaged updates_only", &engaged);
    refuse_every_denied(&engaged, addr);

    let mut offline = KillSwitch::new(NetworkPolicy::offline());
    offline.release();
    report("released offline", &offline);
    refuse_every_denied(&offline, addr);

    drop(listener);
    println!("probe: no dial was allowed");
}

/// Prints the decision for every channel.
fn report(label: &str, switch: &KillSwitch) {
    for feature in NetworkFeature::ALL {
        println!("{label}: {feature} allowed={}", switch.allows(feature));
    }
}

/// Dials every channel the switch denies and asserts nothing was dialed.
fn refuse_every_denied(switch: &KillSwitch, addr: std::net::SocketAddr) {
    for feature in NetworkFeature::ALL {
        if switch.allows(feature) {
            continue;
        }
        assert_blocked(update::dial(switch, feature, addr), feature, "update");
        #[cfg(feature = "net")]
        assert_blocked(
            curbcut_core::net::transport::dial(switch, feature, addr),
            feature,
            "transport",
        );
    }
}

/// Fails unless the dial was refused.
fn assert_blocked(result: Result<TcpStream, NetworkError>, feature: NetworkFeature, path: &str) {
    assert!(
        matches!(result, Err(NetworkError::Blocked)),
        "{path}::dial allowed {feature}; the gate did not hold"
    );
}
