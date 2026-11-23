//! A deterministic property-test and fuzz harness.
//!
//! The workspace builds offline and carries no dependencies, so this is a
//! small seeded generator plus a runner rather than a property-test crate. It
//! exists so every property test in the workspace fails the same way: with the
//! seed and the case index, which is enough to replay the case exactly.
//!
//! Two rules hold:
//!
//! - **A property is an invariant, not an example.** The body runs against
//!   generated input and returns `Err` with a static message when the
//!   invariant breaks. It never asserts one literal input and output pair; that
//!   is what the unit tests are for.
//! - **A failure message carries no content.** It reports the seed, the case
//!   index, and a fixed description. It never includes a generated string, a
//!   screen value, or a key, because a failure message reaches the test log.
//!
//! [`Rng::ascii`] uses a lowercase alphabet for the same reason: a generated
//! string cannot spell a canary prefix, so a fuzz case can never be mistaken
//! for a leak.

use std::fmt;
use std::time::{Duration, Instant};

/// The default number of cases in a property run.
pub const DEFAULT_CASES: u64 = 2_000;

/// The default wall-clock budget for a fuzz run, in milliseconds.
pub const DEFAULT_BUDGET_MS: u64 = 30_000;

/// A deterministic generator, so a failure is reproducible from its seed.
///
/// Splitmix64: one add and three multiply-xor-shift rounds. It is not
/// cryptographic, and it must never be used to generate a key or a nonce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rng {
    seed: u64,
    state: u64,
}

impl Rng {
    /// Creates a generator from a seed.
    pub const fn new(seed: u64) -> Self {
        Self { seed, state: seed }
    }

    /// The seed this generator started from.
    ///
    /// Unlike the internal state, this does not advance, so a failure can
    /// report it and the case can be replayed.
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// The next pseudo-random `u64`.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..bound`. A zero bound returns zero.
    pub fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            0
        } else {
            self.next_u64() % bound
        }
    }

    /// A value in `low..=high`. An empty or inverted range returns `low`.
    ///
    /// The full `u64` range is handled without the `high - low + 1` overflow:
    /// there the span already covers every value, so a raw draw is the answer.
    pub fn between(&mut self, low: u64, high: u64) -> u64 {
        if high <= low {
            return low;
        }
        let span = high - low;
        if span == u64::MAX {
            self.next_u64()
        } else {
            low + self.below(span + 1)
        }
    }

    /// A boolean.
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// One item from `items`, or `None` when the slice is empty.
    pub fn choose<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            Some(&items[self.below(items.len() as u64) as usize])
        }
    }

    /// `len` pseudo-random bytes.
    pub fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.next_u64() & 0xFF) as u8).collect()
    }

    /// `len` characters from a lowercase alphabet.
    ///
    /// The alphabet is lowercase and punctuation-only besides, so a generated
    /// string cannot spell a canary prefix.
    pub fn ascii(&mut self, len: usize) -> String {
        const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789 -_.,:;/\n\t\"'";
        (0..len)
            .map(|_| ALPHABET[self.below(ALPHABET.len() as u64) as usize] as char)
            .collect()
    }

    /// Shuffles a slice in place.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for index in (1..items.len()).rev() {
            let swap = self.below(index as u64 + 1) as usize;
            items.swap(index, swap);
        }
    }
}

/// The first invariant break in a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    seed: u64,
    case: u64,
    message: String,
}

impl Failure {
    /// The seed that reproduces the case.
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// The case index that broke.
    pub const fn case(&self) -> u64 {
        self.case
    }

    /// The static description of the broken invariant.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "property failed at seed {} case {}: {}",
            self.seed, self.case, self.message
        )
    }
}

impl std::error::Error for Failure {}

/// A wall-clock budget for a run.
#[derive(Debug, Clone, Copy)]
pub struct Deadline(Instant);

impl Deadline {
    /// A budget of `budget` from now.
    pub fn new(budget: Duration) -> Self {
        Self(Instant::now() + budget)
    }

    /// Whether the budget is spent.
    pub fn expired(&self) -> bool {
        Instant::now() >= self.0
    }

    /// The budget left. Zero once spent.
    pub fn remaining(&self) -> Duration {
        self.0.saturating_duration_since(Instant::now())
    }
}

/// Runs `property` over `cases` independent cases derived from `seed`.
///
/// Case `n` uses `seed.wrapping_add(n)`, so a failure at case `n` is
/// reproducible on its own. A property that always returns `Ok` passes.
pub fn run<F>(seed: u64, cases: u64, property: F) -> Result<(), Failure>
where
    F: FnMut(&mut Rng, u64) -> Result<(), String>,
{
    run_inner(seed, cases, None, property)
}

/// Runs `property` under a wall-clock budget.
///
/// Running out of budget is not a failure: the budget is a cap so a slow
/// runner cannot time out the suite. CI raises `cases` and lets the budget
/// stop the run.
pub fn run_budgeted<F>(seed: u64, cases: u64, budget: Duration, property: F) -> Result<(), Failure>
where
    F: FnMut(&mut Rng, u64) -> Result<(), String>,
{
    run_inner(seed, cases, Some(Deadline::new(budget)), property)
}

fn run_inner<F>(
    seed: u64,
    cases: u64,
    deadline: Option<Deadline>,
    mut property: F,
) -> Result<(), Failure>
where
    F: FnMut(&mut Rng, u64) -> Result<(), String>,
{
    for case in 0..cases {
        if deadline.is_some_and(|deadline| deadline.expired()) {
            break;
        }
        let mut rng = Rng::new(seed.wrapping_add(case));
        property(&mut rng, case).map_err(|message| Failure {
            seed,
            case,
            message,
        })?;
    }
    Ok(())
}

/// Reads a case budget from the environment, or `default`.
///
/// A local run can raise the budget with `CURBCUT_PROP_CASES`; CI can lower it.
/// A missing or unparseable value is the default, never an error.
pub fn case_budget(env: &str, default: u64) -> u64 {
    std::env::var(env)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

/// Reads a millisecond budget from the environment, or `default`.
pub fn budget_ms(env: &str, default: u64) -> u64 {
    case_budget(env, default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_reproduces_the_same_sequence() {
        let mut left = Rng::new(0x5eed);
        let mut right = Rng::new(0x5eed);
        for _ in 0..64 {
            assert_eq!(left.next_u64(), right.next_u64());
        }
        assert_eq!(left.seed(), 0x5eed);
    }

    #[test]
    fn different_seeds_diverge() {
        let mut left = Rng::new(1);
        let mut right = Rng::new(2);
        let same = (0..16).all(|_| left.next_u64() == right.next_u64());
        assert!(!same, "two distinct seeds produced the same stream");
    }

    #[test]
    fn ranges_are_bounded_and_total() {
        let mut rng = Rng::new(7);
        for _ in 0..1_000 {
            assert!(rng.below(0) == 0);
            assert!(rng.below(1) == 0);
            let value = rng.between(10, 20);
            assert!((10..=20).contains(&value), "{value}");
            assert_eq!(rng.between(5, 5), 5);
            assert_eq!(rng.between(9, 3), 9);
            // The full range exercises the overflow branch.
            let _ = rng.between(0, u64::MAX);
        }
    }

    #[test]
    fn choose_returns_a_member_or_none() {
        let mut rng = Rng::new(11);
        let items = [1_u32, 2, 3, 4];
        for _ in 0..100 {
            let chosen = rng.choose(&items).copied();
            assert!(chosen.is_some_and(|value| items.contains(&value)));
        }
        let empty: [u32; 0] = [];
        assert!(rng.choose(&empty).is_none());
    }

    #[test]
    fn bytes_and_ascii_have_the_requested_length() {
        let mut rng = Rng::new(13);
        assert_eq!(rng.bytes(0).len(), 0);
        assert_eq!(rng.bytes(37).len(), 37);
        assert_eq!(rng.ascii(0).len(), 0);
        assert_eq!(rng.ascii(500).len(), 500);
    }

    #[test]
    fn ascii_is_lowercase_and_carries_no_canary_prefix() {
        let prefix = crate::redact::CANARY_PREFIX;
        let mut rng = Rng::new(17);
        for _ in 0..200 {
            let text = rng.ascii(256);
            assert!(
                !text.contains(prefix),
                "the alphabet spelled the canary prefix"
            );
            for ch in text.chars() {
                assert!(
                    ch.is_ascii_lowercase()
                        || ch.is_ascii_digit()
                        || " -_.,:;/\n\t\"'".contains(ch),
                    "unexpected character {ch:?}"
                );
            }
        }
    }

    #[test]
    fn shuffle_keeps_every_element() {
        let mut rng = Rng::new(19);
        let original: Vec<u32> = (0..64).collect();
        let mut shuffled = original.clone();
        for _ in 0..50 {
            rng.shuffle(&mut shuffled);
            let mut sorted = shuffled.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, original);
        }
    }

    #[test]
    fn a_failing_case_reports_its_seed_and_index() {
        let failure = run(0xabcd, 8, |_, case| {
            if case == 3 {
                Err("the invariant broke".to_string())
            } else {
                Ok(())
            }
        })
        .expect_err("the run must fail");
        assert_eq!(failure.seed(), 0xabcd);
        assert_eq!(failure.case(), 3);
        assert_eq!(failure.message(), "the invariant broke");
        assert!(
            failure.to_string().contains("seed 43981 case 3"),
            "{failure}"
        );
    }

    #[test]
    fn a_passing_property_passes() {
        run(1, 500, |rng, _| {
            let value = rng.between(5, 9);
            if (5..=9).contains(&value) {
                Ok(())
            } else {
                Err("out of range".to_string())
            }
        })
        .expect("the invariant holds");
    }

    #[test]
    fn a_zero_budget_runs_no_cases() {
        let mut calls = 0_u64;
        run_budgeted(1, 1_000, Duration::ZERO, |_, _| {
            calls += 1;
            Ok(())
        })
        .expect("an expired budget is not a failure");
        assert_eq!(calls, 0);
    }

    #[test]
    fn a_deadline_reports_what_is_left() {
        let deadline = Deadline::new(Duration::from_millis(50));
        assert!(!deadline.expired());
        assert!(deadline.remaining() <= Duration::from_millis(50));
    }

    #[test]
    fn budgets_come_from_the_environment() {
        // A unique name, so this test cannot race another reader.
        const NAME: &str = "CURBCUT_PROP_TEST_BUDGET";
        std::env::set_var(NAME, "1234");
        assert_eq!(case_budget(NAME, 1), 1234);
        assert_eq!(budget_ms(NAME, 1), 1234);
        std::env::set_var(NAME, "not a number");
        assert_eq!(case_budget(NAME, 7), 7);
        std::env::remove_var(NAME);
        assert_eq!(case_budget(NAME, 9), 9);
    }
}
