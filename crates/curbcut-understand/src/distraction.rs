//! Interruption scoring.
//!
//! An interruption is scored from its kind and whether it can be dismissed.
//! A modal paywall that cannot be dismissed is the highest band; a dismissible
//! toast is the lowest. The score is a deterministic heuristic, not a model
//! output.

use curbcut_core::types::{Interruption, InterruptionKind};

/// The band an interruption falls into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DistractionBand {
    /// Not worth suppressing.
    None,
    /// Low: worth noting.
    Low,
    /// Medium: worth suppressing under a normal profile.
    Medium,
    /// High: worth suppressing under any profile.
    High,
}

/// A scored interruption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistractionScore {
    /// Score from 0 to 100.
    pub score: u8,
    /// The band the score falls into.
    pub band: DistractionBand,
}

/// Scores an interruption. Pure.
pub fn distraction_score(interruption: &Interruption) -> u8 {
    let base = match interruption.kind {
        InterruptionKind::Toast => 20,
        InterruptionKind::Banner => 40,
        InterruptionKind::CookieNotice => 60,
        InterruptionKind::Popup => 70,
        InterruptionKind::Paywall => 90,
    };
    let penalty = if interruption.dismissible { 0 } else { 20 };
    (base + penalty).min(100)
}

/// Maps a score onto a band.
pub fn band(score: u8) -> DistractionBand {
    match score {
        0..=24 => DistractionBand::None,
        25..=49 => DistractionBand::Low,
        50..=79 => DistractionBand::Medium,
        _ => DistractionBand::High,
    }
}

/// Scores an interruption and its band together.
pub fn score(interruption: &Interruption) -> DistractionScore {
    let score = distraction_score(interruption);
    DistractionScore {
        score,
        band: band(score),
    }
}

/// Whether the rule suppresses this interruption at `threshold`.
pub fn should_suppress(interruption: &Interruption, threshold: u8) -> bool {
    distraction_score(interruption) >= threshold
}

#[cfg(test)]
mod tests {
    use super::*;
    use curbcut_core::types::Rect;

    fn interruption(kind: InterruptionKind, dismissible: bool) -> Interruption {
        Interruption {
            kind,
            rect: Rect::new(0, 0, 10, 10),
            dismissible,
        }
    }

    #[test]
    fn kinds_have_a_stable_order() {
        let dismissible = [
            InterruptionKind::Toast,
            InterruptionKind::Banner,
            InterruptionKind::CookieNotice,
            InterruptionKind::Popup,
            InterruptionKind::Paywall,
        ];
        let mut previous = 0;
        for kind in dismissible {
            let score = distraction_score(&interruption(kind, true));
            assert!(score >= previous, "{kind:?} must not score lower");
            previous = score;
        }
    }

    #[test]
    fn a_non_dismissible_interruption_scores_higher() {
        let toast = interruption(InterruptionKind::Toast, true);
        let stuck_toast = interruption(InterruptionKind::Toast, false);
        assert_eq!(
            distraction_score(&stuck_toast),
            distraction_score(&toast) + 20
        );
    }

    #[test]
    fn bands_follow_the_score() {
        assert_eq!(band(0), DistractionBand::None);
        assert_eq!(band(24), DistractionBand::None);
        assert_eq!(band(25), DistractionBand::Low);
        assert_eq!(band(49), DistractionBand::Low);
        assert_eq!(band(50), DistractionBand::Medium);
        assert_eq!(band(79), DistractionBand::Medium);
        assert_eq!(band(80), DistractionBand::High);
        assert_eq!(band(100), DistractionBand::High);
    }

    #[test]
    fn a_paywall_is_always_high() {
        let dismissible = interruption(InterruptionKind::Paywall, true);
        let stuck = interruption(InterruptionKind::Paywall, false);
        assert_eq!(score(&dismissible).band, DistractionBand::High);
        assert_eq!(score(&stuck).band, DistractionBand::High);
        assert_eq!(distraction_score(&stuck), 100);
    }

    #[test]
    fn the_threshold_decides_suppression() {
        let banner = interruption(InterruptionKind::Banner, true);
        assert!(should_suppress(&banner, 40));
        assert!(!should_suppress(&banner, 41));
    }
}
