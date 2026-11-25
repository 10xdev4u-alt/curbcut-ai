//! Network policy, the runtime kill-switch, and the compiled offline gate.
//!
//! CurbCut is offline first: nothing dials out unless a policy allows the
//! feature *and* the kill-switch is released. Two independent gates enforce
//! that, and either one is enough to stop a connection:
//!
//! - **The compiled gate.** The `net` cargo feature is what puts the optional
//!   transport in the binary at all. It is opt-in (`default = []`), so a build
//!   that does not enable it has no optional transport, and
//!   [`transport_available`] reports which build this is.
//! - **The runtime gate.** [`KillSwitch`] holds a [`NetworkPolicy`] and an
//!   engaged flag. It starts engaged, so a fresh switch denies everything even
//!   with an online policy; [`KillSwitch::release`] hands the decision to the
//!   policy.
//!
//! [`NetworkFeature::UpdateCheck`] is the one channel every build keeps, because
//! a user cannot fix a broken install they cannot download. In a build without
//! the `net` feature the optional transport does not exist, so every other
//! feature has no code path to reach.
//!
//! Nothing here carries screen content. A feature names a channel, and an error
//! names a feature or a static sentence. See
//! [`../../../docs/NETWORK_POLICY.md`](../../../docs/NETWORK_POLICY.md).

use std::fmt;

/// A channel that needs a socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NetworkFeature {
    /// Download a model, a vocabulary, or a voice.
    ModelDownload,
    /// Check whether a newer build exists.
    UpdateCheck,
    /// Sync profiles to the optional encrypted sync service.
    Sync,
    /// Send opt-in telemetry.
    Telemetry,
    /// Validate a licence or a donation key.
    LicenseCheck,
}

impl NetworkFeature {
    /// Every feature, in policy-table order.
    pub const ALL: [NetworkFeature; 5] = [
        Self::ModelDownload,
        Self::UpdateCheck,
        Self::Sync,
        Self::Telemetry,
        Self::LicenseCheck,
    ];

    /// The stable string form used in config and logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ModelDownload => "model_download",
            Self::UpdateCheck => "update_check",
            Self::Sync => "sync",
            Self::Telemetry => "telemetry",
            Self::LicenseCheck => "license_check",
        }
    }

    /// Whether the offline build keeps this channel.
    ///
    /// Only the update check survives: it is how a broken install is repaired.
    pub const fn survives_offline(self) -> bool {
        matches!(self, Self::UpdateCheck)
    }
}

impl fmt::Display for NetworkFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How much network a profile allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NetworkMode {
    /// No channel may dial.
    Offline,
    /// Only the update channel may dial.
    UpdatesOnly,
    /// Every channel may dial, subject to the runtime switch.
    Online,
}

impl NetworkMode {
    /// Every mode, from most to least restrictive.
    pub const ALL: [NetworkMode; 3] = [Self::Offline, Self::UpdatesOnly, Self::Online];

    /// The stable string form used in config and logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Offline => "offline",
            Self::UpdatesOnly => "updates_only",
            Self::Online => "online",
        }
    }

    /// Whether this mode allows one feature.
    pub const fn allows(self, feature: NetworkFeature) -> bool {
        match self {
            Self::Offline => false,
            Self::UpdatesOnly => feature.survives_offline(),
            Self::Online => true,
        }
    }
}

impl fmt::Display for NetworkMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A pure decision: which channels a profile allows.
///
/// The policy is data. It holds no socket, no clock, and no I/O, so the same
/// policy always answers the same question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkPolicy {
    mode: NetworkMode,
}

impl NetworkPolicy {
    /// A policy that allows nothing.
    pub const fn offline() -> Self {
        Self {
            mode: NetworkMode::Offline,
        }
    }

    /// A policy that allows only the update channel.
    pub const fn updates_only() -> Self {
        Self {
            mode: NetworkMode::UpdatesOnly,
        }
    }

    /// A policy that allows every channel.
    pub const fn online() -> Self {
        Self {
            mode: NetworkMode::Online,
        }
    }

    /// A policy for a mode.
    pub const fn from_mode(mode: NetworkMode) -> Self {
        Self { mode }
    }

    /// The mode in force.
    pub const fn mode(&self) -> NetworkMode {
        self.mode
    }

    /// Whether this policy allows one feature.
    pub const fn allows(&self, feature: NetworkFeature) -> bool {
        self.mode.allows(feature)
    }

    /// Whether this policy allows at least one of `features`.
    pub fn allows_any(&self, features: &[NetworkFeature]) -> bool {
        features.iter().any(|feature| self.allows(*feature))
    }

    /// Whether this policy allows every one of `features`.
    ///
    /// An empty list is refused: a request that names no channel is not a
    /// request the policy can grant.
    pub fn allows_all(&self, features: &[NetworkFeature]) -> bool {
        !features.is_empty() && features.iter().all(|feature| self.allows(*feature))
    }
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self::offline()
    }
}

/// The runtime kill-switch.
///
/// A switch is two gates in one value: the engaged flag and the policy. It
/// starts engaged, so constructing one denies everything until
/// [`KillSwitch::release`] is called, and [`KillSwitch::engage`] denies
/// everything again regardless of the policy. Either gate alone is enough to
/// stop a connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillSwitch {
    policy: NetworkPolicy,
    engaged: bool,
}

impl KillSwitch {
    /// Creates a switch over `policy`, engaged.
    pub const fn new(policy: NetworkPolicy) -> Self {
        Self {
            policy,
            engaged: true,
        }
    }

    /// Creates an engaged switch over [`NetworkPolicy::offline`].
    pub const fn engaged() -> Self {
        Self::new(NetworkPolicy::offline())
    }

    /// Builds a switch from a persisted config document.
    ///
    /// A false `update_check` leaves the switch engaged. A true one starts a
    /// fresh engaged switch over [`NetworkPolicy::updates_only`] and releases
    /// it, so only [`NetworkFeature::UpdateCheck`] can dial. The value is read
    /// once, when the switch is built; later config changes are not live.
    /// No shipped binary calls this constructor.
    pub fn from_config(config: &crate::config::Config) -> Self {
        if config.update_check {
            let mut switch = Self::new(NetworkPolicy::updates_only());
            switch.release();
            switch
        } else {
            Self::engaged()
        }
    }

    /// Blocks every channel, whatever the policy says.
    pub fn engage(&mut self) {
        self.engaged = true;
    }

    /// Hands the decision to the policy.
    pub fn release(&mut self) {
        self.engaged = false;
    }

    /// Whether the switch is currently blocking every channel.
    pub const fn is_engaged(&self) -> bool {
        self.engaged
    }

    /// The policy the switch applies once released.
    pub const fn policy(&self) -> NetworkPolicy {
        self.policy
    }

    /// Replaces the policy. The engaged flag is unchanged.
    pub fn set_policy(&mut self, policy: NetworkPolicy) {
        self.policy = policy;
    }

    /// Whether one feature may dial.
    pub const fn allows(&self, feature: NetworkFeature) -> bool {
        !self.engaged && self.policy.allows(feature)
    }
}

impl Default for KillSwitch {
    fn default() -> Self {
        Self::engaged()
    }
}

/// Why a connection was not made.
pub enum NetworkError {
    /// The switch or the policy refused the channel. Nothing was dialed.
    Blocked,
    /// The channel was allowed, but the socket failed.
    ConnectFailed(std::io::Error),
}

impl NetworkError {
    /// The stable string form used in logs and telemetry.
    ///
    /// It names the class, never the address: an address can carry a host name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::ConnectFailed(_) => "connect_failed",
        }
    }
}

impl fmt::Debug for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Blocked => f.write_str("blocked"),
            Self::ConnectFailed(error) => write!(f, "connect_failed: {}", error.kind()),
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for NetworkError {}

/// Whether this build contains the optional transport.
///
/// The `net` cargo feature is the compiled gate. A build without it has no
/// optional transport module at all, so [`NetworkFeature::ModelDownload`],
/// [`NetworkFeature::Sync`], [`NetworkFeature::Telemetry`], and
/// [`NetworkFeature::LicenseCheck`] have no code path that can reach a socket;
/// only [`update::dial`] remains.
pub const fn transport_available() -> bool {
    cfg!(feature = "net")
}

/// The optional transport, compiled in only with the `net` feature.
///
/// This is the only place an optional channel can open a socket. A build
/// without the feature has no `transport` module, so an optional feature cannot
/// be dialled even by a caller that ignores the policy.
#[cfg(feature = "net")]
pub mod transport {
    use std::net::{TcpStream, ToSocketAddrs};

    use super::{KillSwitch, NetworkError, NetworkFeature};

    /// Opens a connection for one optional feature.
    ///
    /// Refuses with [`NetworkError::Blocked`] when the switch denies it. The
    /// switch is consulted here, at the boundary, so a caller cannot forget it.
    pub fn dial<A: ToSocketAddrs>(
        switch: &KillSwitch,
        feature: NetworkFeature,
        addr: A,
    ) -> Result<TcpStream, NetworkError> {
        if !switch.allows(feature) {
            return Err(NetworkError::Blocked);
        }
        TcpStream::connect(addr).map_err(NetworkError::ConnectFailed)
    }
}

/// The update channel, present in every build.
///
/// This is the one path a `--no-default-features` build keeps. It still
/// consults the switch, so an engaged switch blocks updates too, and it accepts
/// only [`NetworkFeature::UpdateCheck`]: a caller cannot route a denied
/// optional channel through the update path.
pub mod update {
    use std::net::{TcpStream, ToSocketAddrs};

    use super::{KillSwitch, NetworkError, NetworkFeature};

    /// Opens a connection for the update check.
    ///
    /// Returns [`NetworkError::Blocked`] for any other feature, before the
    /// switch is consulted, so the channel is classified at the boundary.
    pub fn dial<A: ToSocketAddrs>(
        switch: &KillSwitch,
        feature: NetworkFeature,
        addr: A,
    ) -> Result<TcpStream, NetworkError> {
        if feature != NetworkFeature::UpdateCheck {
            return Err(NetworkError::Blocked);
        }
        if !switch.allows(feature) {
            return Err(NetworkError::Blocked);
        }
        TcpStream::connect(addr).map_err(NetworkError::ConnectFailed)
    }

    /// Dials the update channel using a switch built from a config document.
    ///
    /// This delegates to [`dial`], so the classification and the switch check
    /// live in one place. No shipped binary or UI calls this helper.
    ///
    /// # Errors
    ///
    /// Returns [`NetworkError::Blocked`] when the config-built switch denies
    /// the channel, and [`NetworkError::ConnectFailed`] when the socket fails.
    // Keep the delegation on one line: the policy validator pins this call.
    #[rustfmt::skip]
    pub fn dial_from_config<A: ToSocketAddrs>(
        config: &crate::config::Config,
        addr: A,
    ) -> Result<TcpStream, NetworkError> {
        dial(&KillSwitch::from_config(config), NetworkFeature::UpdateCheck, addr)
    }
}

#[cfg(test)]
mod tests;
