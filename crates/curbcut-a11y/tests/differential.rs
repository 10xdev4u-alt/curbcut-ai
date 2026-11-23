//! Differential test: our reading order and roles against a recorded baseline.
//!
//! `curbcut_a11y::differential::fixtures` is the fixture set, and
//! `differential::measure` scores each fixture. This test asserts the published
//! baseline and prints a triage table, so `cargo test --test differential
//! -- --nocapture` shows exactly which element disagrees and why. See
//! [`../../../docs/DIFFERENTIAL_TESTS.md`](../../../docs/DIFFERENTIAL_TESTS.md).

use curbcut_a11y::differential::{
    fixtures, measure, BASELINE_DISCREPANCIES, BASELINE_ORDER_DISCREPANCIES,
    BASELINE_ROLE_DISCREPANCIES,
};

/// The published baseline: a fix lowers it, a regression raises it and fails.
#[test]
fn the_fixture_set_matches_the_published_baseline() {
    let reports: Vec<_> = fixtures()
        .iter()
        .map(|fixture| {
            (
                fixture.name,
                measure(
                    &fixture.graph,
                    &fixture.baseline_order,
                    &fixture.baseline_roles,
                ),
            )
        })
        .collect();

    let order: usize = reports.iter().map(|(_, report)| report.order.len()).sum();
    let roles: usize = reports.iter().map(|(_, report)| report.roles.len()).sum();
    let total = order + roles;

    println!("differential baseline");
    println!(
        "{:<20} {:>6} {:>6} {:>6}",
        "fixture", "order", "role", "total"
    );
    for (name, report) in &reports {
        println!(
            "{:<20} {:>6} {:>6} {:>6}",
            name,
            report.order.len(),
            report.roles.len(),
            report.len()
        );
    }
    println!("{:<20} {:>6} {:>6} {:>6}", "TOTAL", order, roles, total);
    println!(
        "{:<20} {:>6} {:>6} {:>6}",
        "BASELINE",
        BASELINE_ORDER_DISCREPANCIES,
        BASELINE_ROLE_DISCREPANCIES,
        BASELINE_DISCREPANCIES
    );
    for (name, report) in &reports {
        for discrepancy in &report.order {
            println!("  {name}: {discrepancy:?}");
        }
        for discrepancy in &report.roles {
            println!("  {name}: {discrepancy:?}");
        }
    }

    assert_eq!(
        order, BASELINE_ORDER_DISCREPANCIES,
        "the reading-order discrepancy count changed; fix the order or update the baseline"
    );
    assert_eq!(
        roles, BASELINE_ROLE_DISCREPANCIES,
        "the role discrepancy count changed; fix the mapping or update the baseline"
    );
    assert_eq!(
        total, BASELINE_DISCREPANCIES,
        "the total discrepancy count changed; fix the merge or update the baseline"
    );
}

/// The measurement is a pure function of the fixture: running it twice must
/// give the same report, so the published baseline is stable.
#[test]
fn the_measurement_is_deterministic() {
    for fixture in fixtures() {
        let first = measure(
            &fixture.graph,
            &fixture.baseline_order,
            &fixture.baseline_roles,
        );
        let second = measure(
            &fixture.graph,
            &fixture.baseline_order,
            &fixture.baseline_roles,
        );
        assert_eq!(first, second, "{} is not deterministic", fixture.name);
    }
}

/// A fixture that already agrees is reported as empty, so a zero in the table
/// means "nothing to triage" rather than "not measured".
#[test]
fn an_agreeing_fixture_has_an_empty_report() {
    let flow = fixtures()
        .into_iter()
        .find(|fixture| fixture.name == "document_flow")
        .expect("the document flow fixture exists");
    let report = measure(&flow.graph, &flow.baseline_order, &flow.baseline_roles);
    assert!(report.is_empty());
    assert_eq!(report.len(), 0);
}
