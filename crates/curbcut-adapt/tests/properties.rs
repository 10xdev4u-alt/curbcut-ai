//! Property tests for the adapt crate: profile settings and dim planning.
//!
//! Invariants, not examples. A failure reports the seed and the case index, so
//! the case can be replayed.

use curbcut_adapt::dim::{dim_plan, DimMotion, DimRequest};
use curbcut_adapt::profile::ProfileGraph;
use curbcut_adapt::settings::{SettingKey, SettingValue, ValueKind};
use curbcut_core::id::ProfileId;
use curbcut_core::prop::{self, Rng};
use curbcut_core::types::Rect;

/// A value of the right type for a kind, in range about half the time.
///
/// Out-of-range values matter: a write must be rejected without touching the
/// graph, which is the property under test.
fn random_value(rng: &mut Rng, kind: ValueKind) -> SettingValue {
    match kind {
        ValueKind::Bool => SettingValue::Bool(rng.bool()),
        ValueKind::Int { min, max } => {
            let span = (max - min).unsigned_abs();
            let value = if rng.bool() {
                min + rng.below(span) as i64
            } else {
                rng.below(2_000) as i64 - 1_000
            };
            SettingValue::Int(value)
        }
        ValueKind::Float { min, max } => {
            let value = if rng.bool() {
                min + rng.below(1_000) as f64 / 1_000.0 * (max - min)
            } else {
                rng.below(2_000) as f64 / 1_000.0 - 1_000.0
            };
            SettingValue::Float(value)
        }
        ValueKind::Enum(options) => {
            if options.is_empty() || rng.bool() {
                SettingValue::Text(ascii_len(rng, 0, 8))
            } else {
                SettingValue::Text(options[rng.below(options.len() as u64) as usize].to_string())
            }
        }
        ValueKind::Text => SettingValue::Text(ascii_len(rng, 0, 16)),
        ValueKind::TextList => {
            let count = rng.below(4) as usize;
            let mut names = Vec::with_capacity(count);
            for _ in 0..count {
                names.push(ascii_len(rng, 1, 8));
            }
            SettingValue::TextList(names)
        }
        ValueKind::KeyList => {
            let count = rng.below(4) as usize;
            let mut names = Vec::with_capacity(count);
            for _ in 0..count {
                names.push(ascii_len(rng, 1, 8));
            }
            SettingValue::KeyList(names)
        }
        ValueKind::ReadingLevel => {
            if rng.bool() {
                SettingValue::Text("keep".to_string())
            } else {
                let grade = rng.between(1, 12);
                SettingValue::Text(format!("grade-{grade}"))
            }
        }
    }
}

/// A generated string whose length is drawn before the string, so the two
/// calls do not borrow the generator twice.
fn ascii_len(rng: &mut Rng, low: u64, high: u64) -> String {
    let len = rng.between(low, high) as usize;
    rng.ascii(len)
}

fn random_graph(rng: &mut Rng) -> ProfileGraph {
    if rng.bool() {
        ProfileGraph::new(ProfileId::focus())
    } else {
        ProfileGraph::new(ProfileId::read())
    }
}

#[test]
fn a_successful_setting_write_leaves_a_valid_graph() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x5000, cases, |rng, _| {
        let mut graph = random_graph(rng);
        let key = *rng
            .choose(SettingKey::ALL)
            .expect("the registry is non-empty");
        let value = random_value(rng, key.value_kind());
        let before = graph.settings().clone();

        if key.is_read_only() {
            if graph.apply(key, value).is_ok() {
                return Err("a read-only key accepted a write".to_string());
            }
            if graph.settings() != &before {
                return Err("a rejected read-only write changed the graph".to_string());
            }
            return Ok(());
        }

        match graph.apply(key, value) {
            Ok(()) => {
                graph
                    .validate()
                    .map_err(|_| "an applied graph is invalid")?;
                let stored = graph.get(key);
                key.validate_value(&stored)
                    .map_err(|_| "the stored value violates its declared kind")?;
            }
            Err(_) => {
                if graph.settings() != &before {
                    return Err("a rejected write changed the graph".to_string());
                }
            }
        }
        Ok(())
    })
    .expect("a successful setting write leaves a valid graph");
}

#[test]
fn a_batch_of_writes_is_atomic() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x5001, cases, |rng, _| {
        let mut graph = random_graph(rng);
        let count = rng.between(1, 4) as usize;
        let mut batch: Vec<(SettingKey, SettingValue)> = Vec::new();
        for _ in 0..count {
            let key = *rng
                .choose(SettingKey::ALL)
                .expect("the registry is non-empty");
            let value = random_value(rng, key.value_kind());
            batch.push((key, value));
        }

        let before = graph.settings().clone();
        match graph.apply_many(&batch) {
            Ok(_) => {
                graph
                    .validate()
                    .map_err(|_| "an applied batch left an invalid graph")?;
            }
            Err(_) => {
                if graph.settings() != &before {
                    return Err("a rejected batch changed the graph".to_string());
                }
            }
        }

        // A batch with a read-only key can never commit.
        let mut poisoned = batch.clone();
        poisoned.push((SettingKey::ProfileId, SettingValue::Text("x".to_string())));
        let before = graph.settings().clone();
        if graph.apply_many(&poisoned).is_ok() {
            return Err("a batch with a read-only key committed".to_string());
        }
        if graph.settings() != &before {
            return Err("a rejected batch changed the graph".to_string());
        }
        Ok(())
    })
    .expect("a batch of writes is atomic");
}

#[test]
fn a_reset_restores_the_baseline() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x5002, cases, |rng, _| {
        let mut graph = random_graph(rng);
        let baseline = graph.settings().clone();
        for _ in 0..rng.between(1, 8) as usize {
            let key = *rng
                .choose(SettingKey::ALL)
                .expect("the registry is non-empty");
            let value = random_value(rng, key.value_kind());
            let _ = graph.apply(key, value);
        }
        graph.reset();
        if graph.settings() != &baseline {
            return Err("a reset did not restore the baseline".to_string());
        }
        if graph.is_modified() {
            return Err("a reset graph is still marked modified".to_string());
        }
        Ok(())
    })
    .expect("a reset restores the baseline");
}

/// A rectangle near a screen, sometimes partly outside it.
fn rect_near(rng: &mut Rng, screen: Rect) -> Rect {
    let x = rng.between(0, screen.width as u64 + 100) as i32 - 50;
    let y = rng.between(0, screen.height as u64 + 100) as i32 - 50;
    Rect::new(x, y, rng.between(1, 200) as u32, rng.between(1, 200) as u32)
}

#[test]
fn a_dim_plan_is_exact_non_overlapping_and_protected() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x5003, cases, |rng, _| {
        let screen = Rect::new(0, 0, rng.between(1, 400) as u32, rng.between(1, 400) as u32);
        let focused = rng.bool().then(|| rect_near(rng, screen));
        let except: Vec<Rect> = (0..rng.between(0, 4) as usize)
            .map(|_| rect_near(rng, screen))
            .collect();
        let motion = if rng.bool() {
            DimMotion::Static
        } else {
            DimMotion::Fade {
                duration_ms: rng.between(0, 1_000) as u32,
            }
        };
        let request = DimRequest {
            screen,
            focused,
            except,
            opacity: rng.between(0, 1_000) as f32 / 1_000.0,
            motion,
        };
        let plan = dim_plan(request).map_err(|_| "a well-formed dim request was rejected")?;
        if !plan.has_no_overlap() {
            return Err("two scrim rectangles overlap".to_string());
        }
        if !plan.is_inside(screen) {
            return Err("a scrim rectangle left the screen".to_string());
        }
        if !plan.does_not_cover_any(&plan.protected) {
            return Err("the scrim covers a protected region".to_string());
        }
        if !plan.covers_exactly(screen) {
            return Err("the scrim and the protected regions do not tile the screen".to_string());
        }
        Ok(())
    })
    .expect("a dim plan is exact, non-overlapping, and protected");
}

#[test]
fn an_out_of_range_opacity_is_rejected() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x5004, cases, |rng, _| {
        // A strictly out-of-range value: `-0.0 == 0.0`, so a draw that can be
        // zero would test an in-range input and fail the property by accident.
        let opacity = if rng.bool() {
            -(1.0 + rng.below(1_000) as f32 / 1_000.0)
        } else {
            1.0 + rng.between(1, 1_000) as f32 / 1_000.0
        };
        let request = DimRequest {
            screen: Rect::new(0, 0, 100, 100),
            focused: None,
            except: Vec::new(),
            opacity,
            motion: DimMotion::Static,
        };
        if dim_plan(request).is_ok() {
            return Err("an out-of-range opacity was accepted".to_string());
        }
        Ok(())
    })
    .expect("an out-of-range opacity is rejected");
}
