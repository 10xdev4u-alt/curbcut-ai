//! The deterministic benchmark.
//!
//! The workload is redaction, which the CLI and the diagnostics path share.
//! `bytes` is a deterministic checksum over the redacted output, so a run can be
//! compared across machines. `elapsed_ns` is the only varying field.

use std::fmt;
use std::time::{Duration, Instant};

use curbcut_core::redact::Redactor;

/// The default iteration count.
pub const DEFAULT_ITERATIONS: u64 = 100_000;

const SECRETS: [&str; 2] = ["bench-secret-one", "bench-secret-two"];

const SAMPLE: &str = "label=bench-secret-one value=bench-secret-two count=1";

/// The result of a benchmark run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchStats {
    /// Iterations performed.
    pub iterations: u64,
    /// Wall-clock duration.
    pub elapsed: Duration,
    /// Sum of the redacted lengths. Deterministic for a fixed iteration count.
    pub bytes: u64,
}

impl fmt::Display for BenchStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "bench iterations={} bytes={} elapsed_ns={}",
            self.iterations,
            self.bytes,
            self.elapsed.as_nanos()
        )
    }
}

/// Runs the redaction benchmark `iterations` times.
///
/// A zero iteration count is allowed and yields zero bytes.
pub fn run(iterations: u64) -> BenchStats {
    let mut redactor = Redactor::new();
    redactor.add_secrets(SECRETS);

    let start = Instant::now();
    let mut bytes = 0u64;
    for _ in 0..iterations {
        bytes = bytes.wrapping_add(redactor.redact(SAMPLE).len() as u64);
    }
    let elapsed = start.elapsed();

    BenchStats {
        iterations,
        elapsed,
        bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_byte_count_is_deterministic() {
        let first = run(1_000);
        let second = run(1_000);
        assert_eq!(first.bytes, second.bytes);
        assert_eq!(first.iterations, 1_000);
        assert!(first.bytes > 0);
    }

    #[test]
    fn zero_iterations_is_allowed() {
        let stats = run(0);
        assert_eq!(stats.iterations, 0);
        assert_eq!(stats.bytes, 0);
    }

    #[test]
    fn the_display_carries_counts_not_content() {
        let text = run(5).to_string();
        assert!(text.contains("iterations=5"), "{text}");
        assert!(text.contains("bytes="), "{text}");
        assert!(text.contains("elapsed_ns="), "{text}");
        for secret in SECRETS {
            assert!(!text.contains(secret), "{text}");
        }
    }

    #[test]
    fn the_default_iteration_count_is_sane() {
        assert_eq!(DEFAULT_ITERATIONS, 100_000);
    }
}
