//! Storage schema, migrations, repositories, and key handling.
//!
//! The plan cache is implemented: [`plan_cache::PlanCache`] keeps overlay plans
//! keyed by app, window, and content hash, with a bounded LRU eviction policy,
//! and [`plan_cache::AppMemory`] remembers a user choice per app, but only for
//! apps the user opted into. See
//! [`../../../docs/PLAN_CACHE.md`](../../../docs/PLAN_CACHE.md).
//!
//! [`understanding_cache::UnderstandingCache`] caches encoded understanding
//! results keyed by content hash, model version, and profile, bounded by the
//! same LRU ([`lru::LruCache`]) and sealed with a caller-supplied
//! [`understanding_cache::Cipher`]. See
//! [`../../../docs/UNDERSTANDING_CACHE.md`](../../../docs/UNDERSTANDING_CACHE.md).
//!
//! [`layout_memory::LayoutMemory`] remembers the simplification a user chose
//! for one app and window, keyed by user, app, and window, with a versioned
//! snapshot, per-app and global reset, and layouts that survive an app update.
//! See [`../../../docs/LAYOUT_MEMORY.md`](../../../docs/LAYOUT_MEMORY.md).
//!
//! [`feedback::LearnedSimplification`] records whether the user accepted or
//! rejected a simplification, keyed by a content hash, and moves a
//! simplification level within bounds: a step at a time, only after a minimum
//! number of observations and records, with a bounded undo history and a
//! bounded log. See
//! [`../../../docs/LEARNED_SIMPLIFICATIONS.md`](../../../docs/LEARNED_SIMPLIFICATIONS.md).
//!
//! The rest of the crate is added incrementally; see
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-store";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A bounded least-recently-used cache.
pub mod lru;

/// Bounded plan cache and opt-in per-app memory.
pub mod plan_cache;

/// Per-user, per-app, per-window layout memory.
pub mod layout_memory;

/// Learned simplifications from accept/reject feedback.
pub mod feedback;

/// Sealed understanding-result cache.
pub mod understanding_cache;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-store");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
