//! Protected-content detection and the accessibility-only fallback.
//!
//! Some surfaces must not be analysed: a DRM window renders black, a secure
//! desktop (lock screen, permission prompt) must never be captured at all, and
//! some windows report that they cannot be captured. This module turns those
//! signals into a verdict and a user-facing message, so the pipeline shows an
//! explanation instead of a fabricated result. See
//! [`../../../docs/PROTECTED_CONTENT.md`](../../../docs/PROTECTED_CONTENT.md).
//!
//! Nothing here reads a window. The platform adapter reports what it observed;
//! the policy decides. A message never contains pixels or text from the screen.

use std::fmt;

/// Default luminance at or below which a sample counts as black.
pub const DEFAULT_BLACK_LUMINANCE: u8 = 8;
/// Default fraction of black samples that makes a frame a black frame.
pub const DEFAULT_BLACK_FRACTION: f32 = 0.98;
/// Default number of consecutive black frames before a frame is refused.
pub const DEFAULT_REQUIRED_BLACK_FRAMES: u32 = 2;
/// Default pixel stride used when sampling a frame.
pub const DEFAULT_SAMPLE_STEP: usize = 4;
/// Largest accepted pixel stride.
pub const MAX_SAMPLE_STEP: usize = 64;

/// Why a surface cannot be analysed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtectedReason {
    /// The window is DRM-protected and renders black.
    DrmWindow,
    /// A secure desktop: lock screen, screen saver, or permission prompt.
    SecureDesktop,
    /// The frame is uniformly black, which is how a protected surface renders.
    BlackFrame,
    /// The window reports that it cannot be captured.
    UncapturableWindow,
}

impl ProtectedReason {
    /// The stable string form used in logs and telemetry.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DrmWindow => "drm_window",
            Self::SecureDesktop => "secure_desktop",
            Self::BlackFrame => "black_frame",
            Self::UncapturableWindow => "uncapturable_window",
        }
    }

    /// The user-facing explanation, including the fallback.
    ///
    /// It contains no screen content: the reason name and the fallback only.
    pub fn message(&self) -> String {
        match self {
            Self::DrmWindow => concat!(
                "This window is protected and cannot be captured. ",
                "CurbCut will describe it from accessibility text only."
            )
            .to_string(),
            Self::SecureDesktop => concat!(
                "This is a secure screen, so it is never captured. ",
                "CurbCut will describe it from accessibility text only."
            )
            .to_string(),
            Self::BlackFrame => concat!(
                "The captured frame is blank or protected, so nothing was analysed. ",
                "CurbCut will describe this app from accessibility text only."
            )
            .to_string(),
            Self::UncapturableWindow => concat!(
                "This window cannot be captured. ",
                "CurbCut will describe it from accessibility text only."
            )
            .to_string(),
        }
    }
}

impl fmt::Display for ProtectedReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What the user is offered after a refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fallback {
    /// Describe the app from its accessibility tree; no pixels.
    A11yOnly,
}

impl Fallback {
    /// The stable string form used in logs and telemetry.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::A11yOnly => "a11y_only",
        }
    }
}

impl fmt::Display for Fallback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The result of checking one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureVerdict {
    /// The frame may be analysed.
    Proceed,
    /// The frame must not be analysed.
    Refuse(ProtectedReason),
}

/// A refusal the user can see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectionReport {
    /// Why the surface was refused.
    pub reason: ProtectedReason,
    /// What the user is offered instead.
    pub fallback: Fallback,
    /// The user-facing explanation.
    pub message: String,
}

/// Builds a report for a reason.
pub fn report(reason: ProtectedReason) -> ProtectionReport {
    ProtectionReport {
        reason,
        fallback: Fallback::A11yOnly,
        message: reason.message(),
    }
}

/// Detection thresholds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProtectionConfig {
    /// Luminance at or below which a sample counts as black.
    pub black_luminance: u8,
    /// Fraction of black samples that makes a frame a black frame.
    pub black_fraction: f32,
    /// Consecutive black frames required before a frame is refused.
    pub required_black_frames: u32,
    /// Pixel stride used when sampling.
    pub sample_step: usize,
}

impl Default for ProtectionConfig {
    fn default() -> Self {
        Self {
            black_luminance: DEFAULT_BLACK_LUMINANCE,
            black_fraction: DEFAULT_BLACK_FRACTION,
            required_black_frames: DEFAULT_REQUIRED_BLACK_FRAMES,
            sample_step: DEFAULT_SAMPLE_STEP,
        }
    }
}

impl ProtectionConfig {
    /// Clamps every field to a usable range and falls back to the default on
    /// `NaN`.
    pub fn clamped(mut self) -> Self {
        if !self.black_fraction.is_finite() {
            self.black_fraction = DEFAULT_BLACK_FRACTION;
        }
        self.black_fraction = self.black_fraction.clamp(0.0, 1.0);
        if self.required_black_frames == 0 {
            self.required_black_frames = DEFAULT_REQUIRED_BLACK_FRAMES;
        }
        if self.sample_step == 0 || self.sample_step > MAX_SAMPLE_STEP {
            self.sample_step = DEFAULT_SAMPLE_STEP;
        }
        self
    }
}

/// Rec.709 luminance of one RGBA pixel, 0..=255.
fn luminance(rgba: [u8; 4]) -> u8 {
    let value = 2126 * u32::from(rgba[0]) + 7152 * u32::from(rgba[1]) + 722 * u32::from(rgba[2]);
    // Alpha is ignored: the compositor has already flattened the frame.
    (value / 10_000) as u8
}

/// The fraction of sampled pixels whose luminance is at or below
/// [`ProtectionConfig::black_luminance`].
///
/// `data` is RGBA, four bytes per pixel. A slice shorter than one pixel scores
/// 0.0, so a truncated buffer is never called a black frame.
pub fn black_fraction(data: &[u8], config: &ProtectionConfig) -> f32 {
    let step = config.sample_step * 4;
    if step == 0 || data.len() < 4 {
        return 0.0;
    }
    let mut sampled = 0u32;
    let mut black = 0u32;
    let mut offset = 0usize;
    while offset + 4 <= data.len() {
        let pixel = [
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ];
        sampled += 1;
        if luminance(pixel) <= config.black_luminance {
            black += 1;
        }
        offset += step;
    }
    if sampled == 0 {
        return 0.0;
    }
    black as f32 / sampled as f32
}

/// Whether one frame is a black frame.
pub fn is_black_frame(data: &[u8], config: &ProtectionConfig) -> bool {
    black_fraction(data, config) >= config.black_fraction
}

/// Decides whether a captured frame may be analysed.
///
/// Precedence: a secure desktop is refused before any pixel is read, then an
/// uncapturable window, then an adapter-reported DRM surface, then a black
/// frame. A black frame is refused only after
/// [`ProtectionConfig::required_black_frames`] consecutive black frames, so one
/// dark frame is not a false positive; a non-black frame clears the streak.
pub struct CapturePolicy {
    config: ProtectionConfig,
    secure_desktop: bool,
    uncapturable: bool,
    drm: bool,
    black_streak: u32,
}

impl Default for CapturePolicy {
    fn default() -> Self {
        Self::new(ProtectionConfig::default())
    }
}

impl CapturePolicy {
    /// Creates a policy with `config`.
    pub fn new(config: ProtectionConfig) -> Self {
        Self {
            config: config.clamped(),
            secure_desktop: false,
            uncapturable: false,
            drm: false,
            black_streak: 0,
        }
    }

    /// The effective configuration.
    pub fn config(&self) -> ProtectionConfig {
        self.config
    }

    /// Reports whether a secure desktop is showing. A secure desktop is never
    /// captured: the policy refuses before looking at any frame.
    pub fn set_secure_desktop(&mut self, secure: bool) {
        self.secure_desktop = secure;
    }

    /// Whether a secure desktop was last reported.
    pub fn secure_desktop(&self) -> bool {
        self.secure_desktop
    }

    /// Reports that the frontmost window cannot be captured at all.
    pub fn set_uncapturable(&mut self, uncapturable: bool) {
        self.uncapturable = uncapturable;
    }

    /// Reports an adapter-detected DRM surface.
    pub fn set_drm(&mut self, drm: bool) {
        self.drm = drm;
    }

    /// Consecutive black frames seen so far.
    pub fn black_streak(&self) -> u32 {
        self.black_streak
    }

    /// Checks one frame.
    pub fn observe(&mut self, frame: &[u8]) -> CaptureVerdict {
        if self.secure_desktop {
            return CaptureVerdict::Refuse(ProtectedReason::SecureDesktop);
        }
        if self.uncapturable {
            return CaptureVerdict::Refuse(ProtectedReason::UncapturableWindow);
        }
        if self.drm {
            return CaptureVerdict::Refuse(ProtectedReason::DrmWindow);
        }
        if is_black_frame(frame, &self.config) {
            self.black_streak = self.black_streak.saturating_add(1);
        } else {
            self.black_streak = 0;
        }
        if self.black_streak >= self.config.required_black_frames {
            CaptureVerdict::Refuse(ProtectedReason::BlackFrame)
        } else {
            CaptureVerdict::Proceed
        }
    }

    /// Checks one frame and returns a report when it is refused.
    pub fn observe_report(&mut self, frame: &[u8]) -> Option<ProtectionReport> {
        match self.observe(frame) {
            CaptureVerdict::Proceed => None,
            CaptureVerdict::Refuse(reason) => Some(report(reason)),
        }
    }

    /// Drops the black streak without touching the adapter signals.
    pub fn reset_black_streak(&mut self) {
        self.black_streak = 0;
    }
}

impl fmt::Debug for CapturePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CapturePolicy")
            .field("secure_desktop", &self.secure_desktop)
            .field("uncapturable", &self.uncapturable)
            .field("drm", &self.drm)
            .field("black_streak", &self.black_streak)
            .finish()
    }
}

#[cfg(test)]
mod tests;
