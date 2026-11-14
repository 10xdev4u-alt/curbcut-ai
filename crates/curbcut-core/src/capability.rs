//! Capability tokens.
//!
//! A token proves that the user granted one capability for one session. Tokens
//! are deliberately not `Clone`: a token cannot be copied into another context.
//! They are also not serializable and carry no screen content.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::error::{CurbCutError, Result};
use crate::id::SessionId;

/// A capability the user can grant for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    /// Observe the screen.
    Observe,
    /// Offer assistance without acting.
    Assist,
    /// Perform actions in other applications.
    Act,
    /// Read frame buffers.
    ReadFrames,
    /// Inject input events.
    InjectInput,
    /// Draw the overlay.
    DrawOverlay,
    /// Read the accessibility tree.
    ReadA11yTree,
    /// Write accessibility values.
    WriteA11y,
    /// Read the local store.
    ReadStore,
    /// Use the network.
    ManageNetwork,
}

impl Capability {
    /// The stable string form used in config and logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::Assist => "assist",
            Self::Act => "act",
            Self::ReadFrames => "read_frames",
            Self::InjectInput => "inject_input",
            Self::DrawOverlay => "draw_overlay",
            Self::ReadA11yTree => "read_a11y_tree",
            Self::WriteA11y => "write_a11y",
            Self::ReadStore => "read_store",
            Self::ManageNetwork => "manage_network",
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Proof that one capability was granted for one session.
///
/// Not `Clone`, not `Copy`, not serializable. Construct only through
/// [`Session::grant`] or [`Session::token`].
pub struct CapabilityToken {
    capability: Capability,
    session: SessionId,
    issued: u64,
    epoch: Arc<AtomicU64>,
}

impl CapabilityToken {
    /// The capability this token proves.
    pub fn capability(&self) -> Capability {
        self.capability
    }

    /// The session it belongs to.
    pub fn session(&self) -> SessionId {
        self.session
    }

    /// Whether the token is still valid: the session epoch must not have moved.
    pub fn is_valid(&self) -> bool {
        self.issued == self.epoch.load(Ordering::Acquire)
    }
}

impl fmt::Debug for CapabilityToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CapabilityToken({} for {}, valid={})",
            self.capability,
            self.session,
            self.is_valid()
        )
    }
}

/// A user session and the capabilities granted to it.
///
/// Not `Clone`: the session is the minting authority. Cloning it would let a
/// clone keep a revoked capability and mint fresh tokens for it.
#[derive(Debug)]
pub struct Session {
    id: SessionId,
    granted: BTreeSet<Capability>,
    epoch: Arc<AtomicU64>,
}

impl Session {
    /// Creates a session with no capabilities.
    pub fn new(id: SessionId) -> Self {
        Self {
            id,
            granted: BTreeSet::new(),
            epoch: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Grants a capability and returns its token.
    pub fn grant(&mut self, capability: Capability) -> CapabilityToken {
        self.granted.insert(capability);
        CapabilityToken {
            capability,
            session: self.id,
            issued: self.epoch.load(Ordering::Acquire),
            epoch: Arc::clone(&self.epoch),
        }
    }

    /// Revokes a capability and invalidates existing tokens for it.
    pub fn revoke(&mut self, capability: Capability) {
        if self.granted.remove(&capability) {
            self.epoch.fetch_add(1, Ordering::Release);
        }
    }

    /// Whether a capability is currently granted.
    pub fn has(&self, capability: Capability) -> bool {
        self.granted.contains(&capability)
    }

    /// Returns a token for a granted capability.
    pub fn token(&self, capability: Capability) -> Option<CapabilityToken> {
        if self.has(capability) {
            Some(CapabilityToken {
                capability,
                session: self.id,
                issued: self.epoch.load(Ordering::Acquire),
                epoch: Arc::clone(&self.epoch),
            })
        } else {
            None
        }
    }

    /// The session id.
    pub fn id(&self) -> SessionId {
        self.id
    }
}

/// Requires a valid token for exactly this capability.
pub fn require(token: &CapabilityToken, expected: Capability) -> Result<()> {
    if !token.is_valid() {
        return Err(CurbCutError::PermissionDenied {
            capability: expected,
        });
    }
    if token.capability == expected {
        Ok(())
    } else {
        Err(CurbCutError::PermissionDenied {
            capability: expected,
        })
    }
}

/// Requires a valid token for any of the capabilities.
pub fn require_any(token: &CapabilityToken, expected: &[Capability]) -> Result<()> {
    let Some(&first) = expected.first() else {
        return Err(CurbCutError::InvalidInput {
            field: "capabilities".to_string(),
        });
    };
    if !token.is_valid() {
        return Err(CurbCutError::PermissionDenied { capability: first });
    }
    if expected.contains(&token.capability) {
        Ok(())
    } else {
        Err(CurbCutError::PermissionDenied { capability: first })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SessionId;

    #[test]
    fn deny_by_default() {
        let session = Session::new(SessionId(1));
        assert!(!session.has(Capability::Act));
        assert!(session.token(Capability::Act).is_none());
    }

    #[test]
    fn grant_and_require() {
        let mut session = Session::new(SessionId(1));
        let token = session.grant(Capability::Act);
        require(&token, Capability::Act).unwrap();
        assert!(require(&token, Capability::InjectInput).is_err());
    }

    #[test]
    fn revoke_denies() {
        let mut session = Session::new(SessionId(1));
        let token = session.grant(Capability::Observe);
        session.revoke(Capability::Observe);
        assert!(require(&token, Capability::Observe).is_err());
        assert!(!token.is_valid());
    }

    #[test]
    fn regrant_after_revoke_issues_a_fresh_valid_token() {
        let mut session = Session::new(SessionId(1));
        let old = session.grant(Capability::Observe);
        session.revoke(Capability::Observe);
        let new = session.grant(Capability::Observe);
        assert!(!old.is_valid());
        assert!(new.is_valid());
        require(&new, Capability::Observe).unwrap();
    }

    #[test]
    fn require_any_matches() {
        let mut session = Session::new(SessionId(1));
        let token = session.grant(Capability::ReadStore);
        require_any(&token, &[Capability::Act, Capability::ReadStore]).unwrap();
    }

    #[test]
    fn token_debug_has_no_content() {
        let mut session = Session::new(SessionId(1));
        let token = session.grant(Capability::Observe);
        let dbg = format!("{token:?}");
        assert!(dbg.contains("observe"));
        assert!(!dbg.contains("SECRET"));
    }
}
