//! Errors from the telemetry boundary.

use std::fmt;

/// What went wrong while recording or exporting a snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TelemetryError {
    /// The key is not on the allowlist.
    UnknownKey,
    /// The export is disabled.
    NotEnabled,
    /// A line had no `=` separator.
    MalformedLine,
    /// The schema version is not supported.
    UnsupportedVersion,
}

impl TelemetryError {
    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnknownKey => "unknown-key",
            Self::NotEnabled => "not-enabled",
            Self::MalformedLine => "malformed-line",
            Self::UnsupportedVersion => "unsupported-version",
        }
    }
}

impl fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for TelemetryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_have_stable_names() {
        assert_eq!(TelemetryError::UnknownKey.as_str(), "unknown-key");
        assert_eq!(TelemetryError::NotEnabled.as_str(), "not-enabled");
        assert_eq!(TelemetryError::MalformedLine.as_str(), "malformed-line");
        assert_eq!(
            TelemetryError::UnsupportedVersion.as_str(),
            "unsupported-version"
        );
        assert_eq!(TelemetryError::UnknownKey.to_string(), "unknown-key");
    }

    #[test]
    fn errors_implement_std_error() {
        fn assert_error(error: impl std::error::Error) {
            let _ = error.to_string();
        }
        assert_error(TelemetryError::NotEnabled);
    }
}
