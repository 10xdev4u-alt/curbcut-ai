//! Typed command surface.
//!
//! Every command declares the capability it needs. The registry checks the
//! caller's token before dispatch, so a frontend that lacks a capability cannot
//! reach the operation even if it guesses the command id.

use std::collections::BTreeMap;
use std::fmt;

use crate::capability::{require, Capability, CapabilityToken};
use crate::error::Result;
use crate::id::{ProfileId, TraceId};

/// A command identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandId(pub String);

impl CommandId {
    /// Creates a command id.
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A command. Variants never carry screen content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Read a status report.
    GetStatus,
    /// Read the current overlay plan.
    GetPlan,
    /// Switch the active profile.
    SetProfile(ProfileId),
    /// Enable or disable the overlay.
    SetEnabled(bool),
    /// Panic: hide, stop capture, mute.
    Panic,
    /// Read the privacy report.
    GetPrivacyReport,
}

/// A dispatchable request.
#[derive(Debug, Clone)]
pub struct CommandRequest {
    /// The command id.
    pub id: CommandId,
    /// The command.
    pub command: Command,
    /// Trace id for logging.
    pub trace_id: TraceId,
}

/// Context passed to a handler.
///
/// The session id is derived from the token, so a caller cannot claim a
/// session it does not own.
#[derive(Debug)]
pub struct CommandContext {
    token: CapabilityToken,
}

impl CommandContext {
    /// Creates a context from a capability token.
    pub fn new(token: CapabilityToken) -> Self {
        Self { token }
    }

    /// The caller's session id, taken from the token.
    pub fn session(&self) -> crate::id::SessionId {
        self.token.session()
    }

    /// The caller's capability token.
    pub fn token(&self) -> &CapabilityToken {
        &self.token
    }
}

/// A handler for one command.
pub trait CommandHandler: Send + Sync {
    /// The capability required to dispatch this command.
    fn capability(&self) -> Capability;

    /// Handles the command.
    fn handle(&self, ctx: &CommandContext, command: &Command) -> Result<CommandResponse>;
}

/// A content-free response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResponse {
    /// An acknowledgement.
    Ack,
    /// A status report.
    Status,
    /// A plan was returned.
    Plan,
    /// A privacy report was returned.
    Report,
}

/// A registry that checks capabilities before dispatch.
#[derive(Default)]
pub struct CommandRegistry {
    handlers: BTreeMap<CommandId, Box<dyn CommandHandler>>,
}

impl CommandRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handler, replacing any previous handler for the id.
    pub fn register<H: CommandHandler + 'static>(&mut self, id: CommandId, handler: H) {
        self.handlers.insert(id, Box::new(handler));
    }

    /// The capability required by a command id.
    pub fn requires(&self, id: &CommandId) -> Option<Capability> {
        self.handlers.get(id).map(|h| h.capability())
    }

    /// Dispatches a request after checking the caller's token.
    pub fn dispatch(
        &self,
        ctx: &CommandContext,
        request: &CommandRequest,
    ) -> Result<CommandResponse> {
        let handler = self.handlers.get(&request.id).ok_or_else(|| {
            crate::error::CurbCutError::InvalidInput {
                field: format!("unknown command {}", request.id),
            }
        })?;
        require(ctx.token(), handler.capability())?;
        handler.handle(ctx, &request.command)
    }

    /// Number of registered handlers.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SessionId;

    struct StatusHandler;

    impl CommandHandler for StatusHandler {
        fn capability(&self) -> Capability {
            Capability::Observe
        }

        fn handle(&self, _ctx: &CommandContext, _command: &Command) -> Result<CommandResponse> {
            Ok(CommandResponse::Status)
        }
    }

    #[test]
    fn dispatch_requires_capability() {
        let mut registry = CommandRegistry::new();
        registry.register(CommandId::new("status"), StatusHandler);
        assert_eq!(
            registry.requires(&CommandId::new("status")),
            Some(Capability::Observe)
        );

        let mut session = crate::capability::Session::new(SessionId(1));
        let token = session.grant(Capability::Observe);
        let ctx = CommandContext::new(token);
        assert_eq!(ctx.session(), SessionId(1));
        let request = CommandRequest {
            id: CommandId::new("status"),
            command: Command::GetStatus,
            trace_id: crate::id::TraceId(1),
        };
        assert_eq!(
            registry.dispatch(&ctx, &request).unwrap(),
            CommandResponse::Status
        );
    }

    #[test]
    fn wrong_capability_is_denied() {
        let mut registry = CommandRegistry::new();
        registry.register(CommandId::new("status"), StatusHandler);
        let mut session = crate::capability::Session::new(SessionId(1));
        let token = session.grant(Capability::Act);
        let ctx = CommandContext::new(token);
        let request = CommandRequest {
            id: CommandId::new("status"),
            command: Command::GetStatus,
            trace_id: crate::id::TraceId(1),
        };
        assert!(registry.dispatch(&ctx, &request).is_err());
    }

    #[test]
    fn revoked_token_is_denied_even_with_the_right_capability() {
        let mut registry = CommandRegistry::new();
        registry.register(CommandId::new("status"), StatusHandler);
        let mut session = crate::capability::Session::new(SessionId(1));
        let token = session.grant(Capability::Observe);
        session.revoke(Capability::Observe);
        let ctx = CommandContext::new(token);
        let request = CommandRequest {
            id: CommandId::new("status"),
            command: Command::GetStatus,
            trace_id: crate::id::TraceId(1),
        };
        assert!(registry.dispatch(&ctx, &request).is_err());
    }

    #[test]
    fn unknown_command_is_rejected_before_dispatch() {
        let registry = CommandRegistry::new();
        let mut session = crate::capability::Session::new(SessionId(1));
        let token = session.grant(Capability::Act);
        let ctx = CommandContext::new(token);
        let request = CommandRequest {
            id: CommandId::new("nope"),
            command: Command::GetStatus,
            trace_id: crate::id::TraceId(1),
        };
        assert!(registry.dispatch(&ctx, &request).is_err());
    }
}
