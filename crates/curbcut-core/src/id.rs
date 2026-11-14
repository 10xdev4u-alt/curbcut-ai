//! Strongly typed identifiers.
//!
//! Ids are newtypes so a [`FrameId`] cannot be passed where an [`ElementId`]
//! is expected. They are `Copy` and cheap; they never contain screen content.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

macro_rules! id_newtype {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u64);

        impl $name {
            /// The raw numeric value.
            pub fn get(self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

id_newtype!(FrameId, "A single captured frame.");
id_newtype!(ElementId, "A single UI element.");
id_newtype!(DisplayId, "A connected display.");
id_newtype!(WindowId, "A top-level window.");
id_newtype!(StableId, "An element id that survives across frames.");
id_newtype!(SessionId, "A user session.");
id_newtype!(ChunkId, "A chunk of reading content.");
id_newtype!(TraceId, "A distributed trace identifier.");

/// Generates process-unique [`TraceId`] values.
#[derive(Debug, Default)]
pub struct TraceIdGenerator {
    next: AtomicU64,
}

impl TraceIdGenerator {
    /// Creates a generator that starts at `start`.
    pub fn new(start: u64) -> Self {
        Self {
            next: AtomicU64::new(start),
        }
    }

    /// Returns the next id.
    pub fn next_id(&self) -> TraceId {
        TraceId(self.next.fetch_add(1, Ordering::Relaxed))
    }
}

/// A profile identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProfileId(String);

impl ProfileId {
    /// The built-in focus profile.
    pub fn focus() -> Self {
        Self("focus".to_string())
    }

    /// The built-in reading profile.
    pub fn read() -> Self {
        Self("read".to_string())
    }

    /// The built-in simplify profile.
    pub fn simple() -> Self {
        Self("simple".to_string())
    }

    /// The built-in calm profile.
    pub fn calm() -> Self {
        Self("calm".to_string())
    }

    /// Creates a custom profile id.
    pub fn custom(slug: &str) -> Self {
        Self(format!("custom:{slug}"))
    }

    /// The string form.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is one of the four built-in profiles.
    pub fn is_builtin(&self) -> bool {
        matches!(self.0.as_str(), "focus" | "read" | "simple" | "calm")
    }
}

impl fmt::Display for ProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for ProfileId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// An application identifier, derived from the OS and never from content.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AppId(String);

impl AppId {
    /// Creates an app id from a bundle/executable name.
    pub fn new(slug: &str) -> Self {
        Self(slug.to_lowercase())
    }

    /// The string form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A rule identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuleId(pub String);

impl RuleId {
    /// Creates a rule id.
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_ids_are_unique_and_monotonic() {
        let gen = TraceIdGenerator::new(1);
        let a = gen.next_id();
        let b = gen.next_id();
        assert_ne!(a, b);
        assert!(b.get() > a.get());
    }

    #[test]
    fn profile_ids_recognize_builtins() {
        assert!(ProfileId::focus().is_builtin());
        assert!(ProfileId::calm().is_builtin());
        assert!(!ProfileId::custom("night").is_builtin());
        assert_eq!(ProfileId::read().as_str(), "read");
    }

    #[test]
    fn app_ids_are_normalized() {
        assert_eq!(
            AppId::new("Visual Studio Code").as_str(),
            "visual studio code"
        );
    }

    #[test]
    fn ids_display_without_content() {
        assert_eq!(FrameId(7).to_string(), "FrameId(7)");
        assert_eq!(TraceId(9).to_string(), "TraceId(9)");
    }
}
