//! Error types.
//!
//! Error messages may name fields, operations, and capabilities. They must
//! never carry screen content, OCR text, or raw accessibility strings.

use std::fmt;

use crate::capability::Capability;

/// The result alias used across the workspace.
pub type Result<T> = std::result::Result<T, CurbCutError>;

/// The workspace error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurbCutError {
    /// A capability token was missing or insufficient.
    PermissionDenied {
        /// The capability that was required.
        capability: Capability,
    },
    /// The platform does not support the operation.
    NotSupported {
        /// The feature name.
        feature: String,
    },
    /// An input failed validation. `field` is a field name, never a value.
    InvalidInput {
        /// The offending field or parameter name.
        field: String,
    },
    /// A configuration document is invalid.
    Config {
        /// What is wrong, without the offending value.
        message: String,
    },
    /// A platform call failed.
    Platform {
        /// The operation that failed.
        operation: String,
        /// A platform error code.
        code: i32,
    },
    /// A storage operation failed.
    Store {
        /// The operation that failed.
        operation: String,
        /// A storage error code.
        code: i32,
    },
    /// An internal invariant was violated.
    Internal {
        /// A static description.
        message: String,
    },
}

impl CurbCutError {
    /// A static description with no dynamic content.
    pub fn summary(&self) -> &'static str {
        match self {
            Self::PermissionDenied { .. } => "permission denied",
            Self::NotSupported { .. } => "not supported on this platform",
            Self::InvalidInput { .. } => "invalid input",
            Self::Config { .. } => "invalid configuration",
            Self::Platform { .. } => "platform call failed",
            Self::Store { .. } => "storage operation failed",
            Self::Internal { .. } => "internal error",
        }
    }
}

impl fmt::Display for CurbCutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PermissionDenied { capability } => {
                write!(f, "permission denied: missing {capability}")
            }
            Self::NotSupported { feature } => write!(f, "not supported: {feature}"),
            Self::InvalidInput { field } => write!(f, "invalid input: {field}"),
            Self::Config { message } => write!(f, "invalid configuration: {message}"),
            Self::Platform { operation, code } => {
                write!(f, "platform call failed: {operation} (code {code})")
            }
            Self::Store { operation, code } => {
                write!(f, "storage operation failed: {operation} (code {code})")
            }
            Self::Internal { message } => write!(f, "internal error: {message}"),
        }
    }
}

impl std::error::Error for CurbCutError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_has_no_content() {
        let err = CurbCutError::InvalidInput {
            field: "active_profile".to_string(),
        };
        let text = err.to_string();
        assert!(!text.contains("SECRET"));
        assert!(text.contains("active_profile"));
    }

    #[test]
    fn summaries_are_static() {
        assert_eq!(
            CurbCutError::Internal {
                message: "boom".into()
            }
            .summary(),
            "internal error"
        );
    }
}
