//! Panic-payload probe.
//!
//! Panics inside [`curbcut_core::chaos::guard`] with a canary string built at
//! runtime from the shared fixture, then exits 0. It prints nothing itself.
//!
//! The point is stderr: if the guard ever lets the panic hook run unsuppressed,
//! the default hook writes `panicked at ... <canary>` and
//! `scripts/check_panic_silence.py` fails. Build it and run it by hand to see
//! the harness hold:
//!
//! ```sh
//! cargo build -p curbcut-core --example panic_payload_probe
//! ./target/debug/examples/panic_payload_probe 2>stderr.txt
//! ```
//!
//! A silent run is the expected result. See `docs/CHAOS_TESTS.md`.

use curbcut_core::chaos::guard;

fn main() {
    // Read from the fixture rather than repeating a literal here: the repository
    // scan for content canaries must keep finding exactly one source of truth.
    let canary = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/content_canaries.txt"
    ))
    .lines()
    .map(str::trim)
    .find(|line| !line.is_empty())
    .expect("at least one canary in the fixture");

    let outcome = guard(move || -> () {
        panic!("{canary}");
    });
    assert!(outcome.is_err(), "the guarded panic was caught");
}
