//! Animation policy: a 150 ms cap, reduced motion, and no vestibular triggers.
//!
//! An [`AnimationPolicy`] turns an [`AnimationRequest`] into an
//! [`AnimationPlan`], or refuses it. Three rules hold:
//!
//! - **Nothing exceeds 150 ms.** [`MAX_DURATION_MS`] is the ceiling, and
//!   [`AnimationPolicy::new`] clamps a requested ceiling to it.
//! - **Reduced motion disables non-essential motion.** Under
//!   [`MotionPreference::Reduced`] an essential change is applied instantly and
//!   a non-essential one is refused.
//! - **No vestibular triggers.** Spin, zoom, and parallax are always refused.
//!
//! The policy never sees screen content: it reads a kind, a duration, and an
//! easing. See [`../../../docs/ANIMATION.md`](../../../docs/ANIMATION.md).

use std::time::Duration;

/// The hard ceiling on any animation, in milliseconds.
pub const MAX_DURATION_MS: u16 = 150;

/// Whether the user asked for full or reduced motion.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum MotionPreference {
    /// Motion is allowed.
    #[default]
    Full,
    /// Non-essential motion is disabled.
    Reduced,
}

impl MotionPreference {
    /// Whether full motion is allowed.
    pub const fn is_full(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Whether motion is reduced.
    pub const fn is_reduced(self) -> bool {
        matches!(self, Self::Reduced)
    }
}

/// The kind of motion an animation uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimationKind {
    /// An opacity change.
    Fade,
    /// A position change.
    Slide,
    /// A size change within a small factor.
    Scale,
    /// A rotation.
    Spin,
    /// A zoom toward or away from the content.
    Zoom,
    /// Movement driven by scroll or pointer position.
    Parallax,
    /// A repeating opacity or size pulse.
    Pulse,
    /// A highlight that does not move.
    Highlight,
}

impl AnimationKind {
    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fade => "fade",
            Self::Slide => "slide",
            Self::Scale => "scale",
            Self::Spin => "spin",
            Self::Zoom => "zoom",
            Self::Parallax => "parallax",
            Self::Pulse => "pulse",
            Self::Highlight => "highlight",
        }
    }

    /// Whether the motion is essential to understand the change.
    ///
    /// A fade and a static highlight carry meaning; the rest are decoration.
    pub const fn is_essential(self) -> bool {
        matches!(self, Self::Fade | Self::Highlight)
    }

    /// Whether the motion is a known vestibular trigger.
    pub const fn is_vestibular_trigger(self) -> bool {
        matches!(self, Self::Spin | Self::Zoom | Self::Parallax)
    }

    /// Whether the motion repeats until stopped.
    pub const fn is_repeating(self) -> bool {
        matches!(self, Self::Pulse)
    }
}

/// How the value changes over the duration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Easing {
    /// A constant rate.
    Linear,
    /// Starts slowly.
    EaseIn,
    /// Settles slowly.
    #[default]
    EaseOut,
    /// Starts and settles slowly.
    EaseInOut,
}

impl Easing {
    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::EaseIn => "ease-in",
            Self::EaseOut => "ease-out",
            Self::EaseInOut => "ease-in-out",
        }
    }
}

/// A requested animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnimationRequest {
    kind: AnimationKind,
    duration_ms: u32,
    easing: Easing,
}

impl AnimationRequest {
    /// Creates a request.
    pub const fn new(kind: AnimationKind, duration_ms: u32) -> Self {
        Self {
            kind,
            duration_ms,
            easing: Easing::EaseOut,
        }
    }

    /// Sets the easing.
    pub const fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// The kind of motion.
    pub const fn kind(self) -> AnimationKind {
        self.kind
    }

    /// The requested duration in milliseconds.
    pub const fn duration_ms(self) -> u32 {
        self.duration_ms
    }

    /// The easing.
    pub const fn easing(self) -> Easing {
        self.easing
    }
}

/// An animation the policy allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnimationPlan {
    kind: AnimationKind,
    duration_ms: u16,
    easing: Easing,
}

impl AnimationPlan {
    /// The kind of motion.
    pub const fn kind(self) -> AnimationKind {
        self.kind
    }

    /// The allowed duration in milliseconds, never above
    /// [`MAX_DURATION_MS`].
    pub const fn duration_ms(self) -> u16 {
        self.duration_ms
    }

    /// The easing.
    pub const fn easing(self) -> Easing {
        self.easing
    }

    /// Whether the change is applied at once.
    pub const fn is_instant(self) -> bool {
        self.duration_ms == 0
    }

    /// The duration, or `None` when the change is applied at once.
    pub fn duration(self) -> Option<Duration> {
        if self.is_instant() {
            None
        } else {
            Some(Duration::from_millis(u64::from(self.duration_ms)))
        }
    }
}

/// The policy every animation passes through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationPolicy {
    max_duration_ms: u16,
    preference: MotionPreference,
}

impl Default for AnimationPolicy {
    fn default() -> Self {
        Self {
            max_duration_ms: MAX_DURATION_MS,
            preference: MotionPreference::Full,
        }
    }
}

impl AnimationPolicy {
    /// Creates a policy.
    ///
    /// `max_duration_ms` is clamped to [`MAX_DURATION_MS`], so a caller cannot
    /// raise the cap.
    pub const fn new(max_duration_ms: u16, preference: MotionPreference) -> Self {
        let max_duration_ms = if max_duration_ms > MAX_DURATION_MS {
            MAX_DURATION_MS
        } else {
            max_duration_ms
        };
        Self {
            max_duration_ms,
            preference,
        }
    }

    /// Creates a policy at the default cap.
    pub const fn from_preference(preference: MotionPreference) -> Self {
        Self {
            max_duration_ms: MAX_DURATION_MS,
            preference,
        }
    }

    /// The ceiling in milliseconds.
    pub const fn max_duration_ms(self) -> u16 {
        self.max_duration_ms
    }

    /// The motion preference.
    pub const fn preference(self) -> MotionPreference {
        self.preference
    }

    /// Whether a kind may animate at all under this policy.
    pub const fn allows(self, kind: AnimationKind) -> bool {
        if kind.is_vestibular_trigger() {
            return false;
        }
        if self.preference.is_reduced() {
            return kind.is_essential();
        }
        true
    }

    /// Plans an animation, or refuses it with `None`.
    ///
    /// A refusal means "apply the end state at once"; it never means "skip the
    /// change". A vestibular trigger is refused under either preference. Under
    /// reduced motion a non-essential kind is refused and an essential one is
    /// planned instantly.
    pub fn plan(self, request: AnimationRequest) -> Option<AnimationPlan> {
        if !self.allows(request.kind) {
            return None;
        }
        let duration_ms = if self.preference.is_reduced() {
            0
        } else {
            request.duration_ms.min(u32::from(self.max_duration_ms))
        };
        Some(AnimationPlan {
            kind: request.kind,
            duration_ms: duration_ms as u16,
            easing: request.easing,
        })
    }
}

#[cfg(test)]
mod tests;
