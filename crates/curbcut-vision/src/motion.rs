//! Motion signatures.
//!
//! The scheduler must decide whether a frame changed without retaining pixels.
//! A [`MotionSignature`] is a fixed-size sample of the frame, kept in memory
//! only. It is never logged, never stored, and its `Debug` prints the sample
//! count rather than the bytes.

use std::fmt;

/// Number of samples in a signature.
pub const MOTION_SAMPLES: usize = 32;

/// A compact, in-memory proxy for a frame's pixels.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct MotionSignature {
    samples: [u8; MOTION_SAMPLES],
}

impl MotionSignature {
    /// Samples `pixels` at evenly spaced positions.
    ///
    /// An empty frame yields an all-zero signature.
    pub fn from_pixels(pixels: &[u8]) -> Self {
        let mut samples = [0u8; MOTION_SAMPLES];
        if pixels.is_empty() {
            return Self { samples };
        }
        for (slot, index) in sample_indices(pixels.len()).enumerate() {
            samples[slot] = pixels[index];
        }
        Self { samples }
    }

    /// The fraction of samples that differ from `previous`, from 0.0 to 1.0.
    pub fn score_against(&self, previous: &Self) -> f32 {
        let changed = self
            .samples
            .iter()
            .zip(previous.samples.iter())
            .filter(|(now, then)| now != then)
            .count();
        changed as f32 / MOTION_SAMPLES as f32
    }
}

impl fmt::Debug for MotionSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MotionSignature")
            .field("samples", &MOTION_SAMPLES)
            .finish()
    }
}

/// The sampled byte index for each of the [`MOTION_SAMPLES`] slots.
fn sample_indices(len: usize) -> impl Iterator<Item = usize> {
    (0..MOTION_SAMPLES).map(move |slot| (slot * len) / MOTION_SAMPLES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_frame_has_a_zero_signature() {
        let signature = MotionSignature::from_pixels(&[]);
        assert!(signature.samples.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn an_identical_frame_scores_zero() {
        let pixels = vec![7u8; 1024];
        let first = MotionSignature::from_pixels(&pixels);
        let second = MotionSignature::from_pixels(&pixels);
        assert_eq!(first.score_against(&second), 0.0);
    }

    #[test]
    fn a_fully_changed_frame_scores_one() {
        let first = MotionSignature::from_pixels(&vec![0u8; 1024]);
        let second = MotionSignature::from_pixels(&vec![1u8; 1024]);
        assert_eq!(first.score_against(&second), 1.0);
    }

    #[test]
    fn a_partial_change_scores_the_changed_fraction() {
        let mut pixels = vec![0u8; 1024];
        // Change exactly one sampled position.
        pixels[sample_indices(1024).next().expect("index")] = 1;
        let first = MotionSignature::from_pixels(&vec![0u8; 1024]);
        let second = MotionSignature::from_pixels(&pixels);
        let score = first.score_against(&second);
        assert!(score > 0.0 && score < 1.0, "score was {score}");
    }

    #[test]
    fn a_short_frame_is_sampled_without_panicking() {
        let signature = MotionSignature::from_pixels(&[1, 2, 3]);
        assert_eq!(signature.samples.len(), MOTION_SAMPLES);
    }

    #[test]
    fn debug_prints_no_samples() {
        let signature = MotionSignature::from_pixels(&[9u8; 64]);
        let dbg = format!("{signature:?}");
        assert!(dbg.contains("samples"), "{dbg}");
        assert!(!dbg.contains("9, 9"), "{dbg}");
    }

    #[test]
    fn signatures_are_deterministic() {
        let pixels = vec![3u8; 256];
        assert_eq!(
            MotionSignature::from_pixels(&pixels),
            MotionSignature::from_pixels(&pixels)
        );
    }
}
