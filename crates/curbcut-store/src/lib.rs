//! Storage schema, migrations, repositories, and key handling.
//!
//! The plan cache is implemented: [`plan_cache::PlanCache`] keeps overlay plans
//! keyed by app, window, and content hash, with a bounded LRU eviction policy,
//! and [`plan_cache::AppMemory`] remembers a user choice per app, but only for
//! apps the user opted into. See
//! [`../../../docs/PLAN_CACHE.md`](../../../docs/PLAN_CACHE.md).
//!
//! The rest of the crate is added incrementally; see
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-store";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Bounded plan cache and opt-in per-app memory.
pub mod plan_cache;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-store");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
