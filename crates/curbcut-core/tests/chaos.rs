//! Chaos suites: failure injection over generated scripts.
//!
//! Each case draws a fresh script from the seed, injects every fault into a
//! component built from real core primitives, and checks the three invariants:
//! no panic escapes the run, every injected fault is announced, and every
//! announcement ends in recovery. A failure reports the seed and the case
//! index, so the script replays exactly.

use curbcut_core::capability::Capability;
use curbcut_core::chaos::{
    CaptureLink, ChaosReport, ChecksumCache, Degradation, Fault, Faulty, PermissionGate,
    ScreenBudget, Script,
};
use curbcut_core::prop::{self, Rng};
use curbcut_core::types::Rect;

/// The three invariants as one check.
fn invariants(report: &ChaosReport) -> Result<(), String> {
    if !report.no_panic_escaped() {
        return Err("a step panicked and the harness did not contain it".to_string());
    }
    if !report.announced_every_fault() {
        return Err("an injected fault produced no degradation".to_string());
    }
    if !report.all_recovered() {
        return Err("a degradation did not recover".to_string());
    }
    Ok(())
}

/// Draws a script length before building the script, so the two calls do not
/// borrow the generator twice.
fn random_script(rng: &mut Rng, low: u64, high: u64) -> Script {
    let steps = rng.between(low, high);
    Script::random(rng, steps)
}

#[test]
fn a_revoked_permission_is_always_announced_and_recovered() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x6000, cases, |rng, _| {
        let mut gate = PermissionGate::new(Capability::Observe);
        let script = random_script(rng, 0, 24);
        let report = curbcut_core::chaos::run(&mut gate, &script);
        invariants(&report)?;
        if !gate.is_usable() {
            return Err("the gate did not return to usable".to_string());
        }
        Ok(())
    })
    .expect("a revoked permission is always announced and recovered");
}

#[test]
fn a_corrupted_cache_entry_never_reads_as_a_value() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x6001, cases, |rng, _| {
        let mut cache = ChecksumCache::new();
        let len = rng.between(1, 64) as usize;
        cache.put(1, &rng.bytes(len));
        let before = cache.get(1).map(<[u8]>::to_vec);

        let script = random_script(rng, 0, 24);
        let report = curbcut_core::chaos::run(&mut cache, &script);
        invariants(&report)?;

        // An entry that comes back must be byte-for-byte the one written.
        if let (Some(expected), Some(actual)) = (&before, cache.get(1)) {
            if actual != expected.as_slice() {
                return Err("a corrupted entry read back different bytes".to_string());
            }
        }
        Ok(())
    })
    .expect("a corrupted cache entry never reads as a value");
}

#[test]
fn a_huge_display_stays_representable_and_recovers() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x6002, cases, |rng, _| {
        let mut screen = ScreenBudget::default();
        let baseline = screen.rect();
        let script = random_script(rng, 0, 24);
        let report = curbcut_core::chaos::run(&mut screen, &script);
        invariants(&report)?;
        if screen.rect() != baseline {
            return Err("the display did not return to its baseline".to_string());
        }
        if screen.total_pixels() != u64::from(baseline.width) * u64::from(baseline.height) {
            return Err("the baseline pixel count changed".to_string());
        }
        Ok(())
    })
    .expect("a huge display stays representable and recovers");
}

#[test]
fn a_remote_desktop_link_degrades_then_recovers() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x6003, cases, |rng, _| {
        let mut link = CaptureLink::Local;
        let script = random_script(rng, 0, 24);
        let report = curbcut_core::chaos::run(&mut link, &script);
        invariants(&report)?;
        if link != CaptureLink::Local {
            return Err("capture did not return to the local link".to_string());
        }
        Ok(())
    })
    .expect("a remote desktop link degrades then recovers");
}

#[test]
fn every_scheduled_fault_leaves_an_announcement() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x6004, cases, |rng, _| {
        let mut gate = PermissionGate::new(Capability::ReadA11yTree);
        let script = random_script(rng, 1, 32);
        let report = curbcut_core::chaos::run(&mut gate, &script);

        // The gate accepts exactly one fault kind, so the accepted count is the
        // announced count. Every announcement carries the fault and a reason.
        let expected = script
            .iter()
            .filter(|(_, fault)| *fault == Fault::PermissionRevoked)
            .count() as u64;
        if report.faults() != expected {
            return Err("the run injected a fault the gate does not accept".to_string());
        }
        if report.degradations().len() as u64 != expected {
            return Err("an accepted fault left no announcement".to_string());
        }
        for degradation in report.degradations() {
            if degradation.fault != Fault::PermissionRevoked {
                return Err("an announcement names the wrong fault".to_string());
            }
            if degradation.reason.is_empty() {
                return Err("an announcement has no reason code".to_string());
            }
        }
        invariants(&report)?;
        Ok(())
    })
    .expect("every scheduled fault leaves an announcement");
}

#[test]
fn extreme_geometry_never_panics() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x6005, cases, |rng, _| {
        let x = rng.next_u64() as i32;
        let y = rng.next_u64() as i32;
        let width = rng.below(u32::MAX as u64) as u32;
        let height = rng.below(u32::MAX as u64) as u32;
        let rect = Rect::new(x, y, width, height);

        // Saturated edges and the area must come back without unwinding.
        let right = curbcut_core::chaos::guard(|| rect.right());
        let bottom = curbcut_core::chaos::guard(|| rect.bottom());
        let area = curbcut_core::chaos::guard(|| rect.area());
        right.map_err(|_| "right edge panicked")?;
        bottom.map_err(|_| "bottom edge panicked")?;
        area.map_err(|_| "area panicked")?;

        let right = right.expect("checked above");
        let bottom = bottom.expect("checked above");
        if right < x || bottom < y {
            return Err("a saturated edge moved backwards".to_string());
        }
        let area = area.expect("checked above");
        if area > u64::from(u32::MAX) * u64::from(u32::MAX) {
            return Err("the area exceeded the representable range".to_string());
        }
        Ok(())
    })
    .expect("extreme geometry never panics");
}

/// A component that panics on injection, proving the harness contains it.
struct Panicky;

impl Faulty for Panicky {
    fn accepts(&self, fault: Fault) -> bool {
        fault == Fault::CacheCorrupt
    }

    fn inject(&mut self, _fault: Fault) {
        panic!("injected");
    }

    fn degradation(&self) -> Option<Degradation> {
        None
    }

    fn recover(&mut self) -> bool {
        true
    }
}

#[test]
fn a_panic_is_caught_counted_and_never_escapes() {
    let mut panicky = Panicky;
    let report = curbcut_core::chaos::run(
        &mut panicky,
        &Script::new([
            (0, Fault::CacheCorrupt),
            (1, Fault::CacheCorrupt),
            (2, Fault::PermissionRevoked),
        ]),
    );
    // The run returned at all, which is the point: the panic did not escape.
    assert_eq!(report.panics(), 1, "{report}");
    assert_eq!(report.faults(), 0, "the run stops at the panic");
    assert!(!report.no_panic_escaped());
}

#[test]
fn a_report_that_meets_every_invariant_is_clean() {
    let mut gate = PermissionGate::new(Capability::Assist);
    let report = curbcut_core::chaos::run(
        &mut gate,
        &Script::new([(0, Fault::PermissionRevoked), (1, Fault::PermissionRevoked)]),
    );
    assert!(report.is_clean(), "{report}");
    assert_eq!(report.steps(), 2);
    assert_eq!(report.faults(), 2);
    assert_eq!(report.degradations().len(), 2);
}
