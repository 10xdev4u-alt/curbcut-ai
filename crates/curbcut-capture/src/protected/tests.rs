use super::*;

const CANARY: &str = "CC-CANARY-TEXT-9f86d081884c7d65";

fn config() -> ProtectionConfig {
    ProtectionConfig {
        black_luminance: 8,
        black_fraction: 0.98,
        required_black_frames: 2,
        sample_step: 1,
    }
}

/// A frame of `count` pixels, each `(r, g, b)`.
fn frame(pixels: &[(u8, u8, u8)]) -> Vec<u8> {
    let mut data = Vec::with_capacity(pixels.len() * 4);
    for &(r, g, b) in pixels {
        data.extend_from_slice(&[r, g, b, 255]);
    }
    data
}

fn black_frame(pixels: usize) -> Vec<u8> {
    frame(&vec![(0, 0, 0); pixels])
}

#[test]
fn luminance_matches_rec709() {
    assert_eq!(super::luminance([0, 0, 0, 255]), 0);
    assert_eq!(super::luminance([255, 255, 255, 255]), 255);
    // 7152 / 10000 = 0.7152, the Rec.709 green weight.
    assert_eq!(super::luminance([0, 255, 0, 255]), 182);
    // Alpha is ignored.
    assert_eq!(super::luminance([255, 255, 255, 0]), 255);
}

#[test]
fn a_black_frame_is_detected() {
    let cfg = config();
    assert!(is_black_frame(&black_frame(64), &cfg));
    assert_eq!(black_fraction(&black_frame(64), &cfg), 1.0);
}

#[test]
fn a_white_frame_is_not_black() {
    let cfg = config();
    let data = frame(&[(255, 255, 255); 64]);
    assert!(!is_black_frame(&data, &cfg));
    assert_eq!(black_fraction(&data, &cfg), 0.0);
}

#[test]
fn a_mostly_black_frame_below_the_threshold_is_not_black() {
    let cfg = ProtectionConfig {
        black_fraction: 0.99,
        sample_step: 1,
        ..config()
    };
    let mut pixels = vec![(0, 0, 0); 98];
    pixels.extend_from_slice(&[(255, 255, 255); 2]);
    assert_eq!(black_fraction(&frame(&pixels), &cfg), 0.98);
    assert!(!is_black_frame(&frame(&pixels), &cfg));
}

#[test]
fn a_truncated_buffer_is_never_a_black_frame() {
    let cfg = config();
    assert_eq!(black_fraction(&[], &cfg), 0.0);
    assert_eq!(black_fraction(&[0, 0, 0], &cfg), 0.0);
    assert!(!is_black_frame(&[0, 0, 0], &cfg));
}

#[test]
fn a_short_buffer_is_refused_by_the_policy() {
    let mut policy = CapturePolicy::new(config());
    assert_eq!(
        policy.observe(&[]),
        CaptureVerdict::Refuse(ProtectedReason::BlackFrame),
        "an empty buffer is not a frame and must not be cleared"
    );
    assert_eq!(
        policy.observe(&[0, 0, 0]),
        CaptureVerdict::Refuse(ProtectedReason::BlackFrame)
    );
    assert_eq!(policy.black_streak(), 0);
}

#[test]
fn one_dark_frame_is_not_a_refusal() {
    let mut policy = CapturePolicy::new(config());
    assert_eq!(
        policy.observe(&black_frame(32)),
        CaptureVerdict::Proceed,
        "a single black frame must not be called protected"
    );
    assert_eq!(policy.black_streak(), 1);
}

#[test]
fn a_streak_of_black_frames_is_refused_then_recovers() {
    let mut policy = CapturePolicy::new(config());
    assert_eq!(policy.observe(&black_frame(32)), CaptureVerdict::Proceed);
    assert_eq!(
        policy.observe(&black_frame(32)),
        CaptureVerdict::Refuse(ProtectedReason::BlackFrame)
    );
    assert_eq!(
        policy.observe(&black_frame(32)),
        CaptureVerdict::Refuse(ProtectedReason::BlackFrame)
    );
    assert_eq!(
        policy.observe(&frame(&[(255, 255, 255); 32])),
        CaptureVerdict::Proceed,
        "a bright frame clears the streak"
    );
    assert_eq!(policy.black_streak(), 0);
    assert_eq!(policy.observe(&black_frame(32)), CaptureVerdict::Proceed);
}

#[test]
fn a_secure_desktop_is_refused_before_any_pixel_is_read() {
    let mut policy = CapturePolicy::new(config());
    policy.set_secure_desktop(true);
    assert!(policy.secure_desktop());
    assert_eq!(
        policy.observe(&frame(&[(255, 255, 255); 32])),
        CaptureVerdict::Refuse(ProtectedReason::SecureDesktop)
    );
    assert_eq!(policy.black_streak(), 0, "no frame was inspected");
}

#[test]
fn a_signal_gap_breaks_the_streak() {
    let mut policy = CapturePolicy::new(config());
    assert_eq!(policy.observe(&black_frame(32)), CaptureVerdict::Proceed);
    assert_eq!(policy.black_streak(), 1);

    // A secure desktop refuses without inspecting a frame, so the streak from
    // before it cannot count as consecutive with a frame after it.
    policy.set_secure_desktop(true);
    assert_eq!(
        policy.observe(&frame(&[(255, 255, 255); 32])),
        CaptureVerdict::Refuse(ProtectedReason::SecureDesktop)
    );
    assert_eq!(policy.black_streak(), 0);
    policy.set_secure_desktop(false);

    assert_eq!(
        policy.observe(&black_frame(32)),
        CaptureVerdict::Proceed,
        "one dark frame after unlocking is not protected"
    );
    assert_eq!(policy.black_streak(), 1);
}

#[test]
fn a_drm_gap_breaks_the_streak() {
    let mut policy = CapturePolicy::new(config());
    assert_eq!(policy.observe(&black_frame(32)), CaptureVerdict::Proceed);
    policy.set_drm(true);
    assert_eq!(
        policy.observe(&frame(&[(255, 255, 255); 32])),
        CaptureVerdict::Refuse(ProtectedReason::DrmWindow)
    );
    assert_eq!(policy.black_streak(), 0);
    policy.set_drm(false);
    assert_eq!(policy.observe(&black_frame(32)), CaptureVerdict::Proceed);
}

#[test]
fn an_uncapturable_window_is_refused() {
    let mut policy = CapturePolicy::new(config());
    policy.set_uncapturable(true);
    assert_eq!(
        policy.observe(&frame(&[(255, 255, 255); 32])),
        CaptureVerdict::Refuse(ProtectedReason::UncapturableWindow)
    );
}

#[test]
fn an_adapter_reported_drm_surface_is_refused() {
    let mut policy = CapturePolicy::new(config());
    policy.set_drm(true);
    assert_eq!(
        policy.observe(&frame(&[(255, 255, 255); 32])),
        CaptureVerdict::Refuse(ProtectedReason::DrmWindow)
    );
}

#[test]
fn a_secure_desktop_outranks_every_other_signal() {
    let mut policy = CapturePolicy::new(config());
    policy.set_secure_desktop(true);
    policy.set_uncapturable(true);
    policy.set_drm(true);
    assert_eq!(
        policy.observe(&frame(&[(255, 255, 255); 32])),
        CaptureVerdict::Refuse(ProtectedReason::SecureDesktop)
    );
}

#[test]
fn a_report_explains_the_fallback() {
    let mut policy = CapturePolicy::new(config());
    policy.set_drm(true);
    let report = policy
        .observe_report(&frame(&[(255, 255, 255); 32]))
        .expect("a refusal report");
    assert_eq!(report.reason, ProtectedReason::DrmWindow);
    assert_eq!(report.fallback, Fallback::A11yOnly);
    assert!(report.message.contains("protected"));
    assert!(report.message.contains("accessibility text only"));
    assert_eq!(report.message, ProtectedReason::DrmWindow.message());
}

#[test]
fn an_allowed_frame_has_no_report() {
    let mut policy = CapturePolicy::new(config());
    assert!(policy
        .observe_report(&frame(&[(255, 255, 255); 32]))
        .is_none());
}

#[test]
fn every_reason_names_a_fallback_and_carries_no_content() {
    for reason in [
        ProtectedReason::DrmWindow,
        ProtectedReason::SecureDesktop,
        ProtectedReason::BlackFrame,
        ProtectedReason::UncapturableWindow,
    ] {
        let report = report(reason);
        assert_eq!(report.reason, reason);
        assert_eq!(report.fallback, Fallback::A11yOnly);
        assert!(!report.message.is_empty());
        assert!(!report.message.contains(CANARY));
        assert!(!report.message.contains("SECRET"));
        assert_eq!(format!("{reason}"), reason.as_str());
    }
    assert_eq!(ProtectedReason::DrmWindow.as_str(), "drm_window");
    assert_eq!(ProtectedReason::SecureDesktop.as_str(), "secure_desktop");
    assert_eq!(ProtectedReason::BlackFrame.as_str(), "black_frame");
    assert_eq!(
        ProtectedReason::UncapturableWindow.as_str(),
        "uncapturable_window"
    );
    assert_eq!(Fallback::A11yOnly.as_str(), "a11y_only");
}

#[test]
fn the_config_is_clamped() {
    let cfg = ProtectionConfig {
        black_fraction: f32::NAN,
        required_black_frames: 0,
        sample_step: 0,
        black_luminance: 200,
    }
    .clamped();
    assert_eq!(cfg.black_fraction, DEFAULT_BLACK_FRACTION);
    assert_eq!(cfg.required_black_frames, DEFAULT_REQUIRED_BLACK_FRAMES);
    assert_eq!(cfg.sample_step, DEFAULT_SAMPLE_STEP);
    assert_eq!(cfg.black_luminance, 200);

    let high = ProtectionConfig {
        black_fraction: 7.5,
        sample_step: MAX_SAMPLE_STEP + 1,
        ..ProtectionConfig::default()
    }
    .clamped();
    assert_eq!(high.black_fraction, 1.0);
    assert_eq!(high.sample_step, DEFAULT_SAMPLE_STEP);
}

#[test]
fn a_zero_black_fraction_falls_back_to_the_default() {
    let cfg = ProtectionConfig {
        black_fraction: 0.0,
        ..config()
    }
    .clamped();
    assert_eq!(cfg.black_fraction, DEFAULT_BLACK_FRACTION);
    assert!(!is_black_frame(&frame(&[(255, 255, 255); 32]), &cfg));
}

#[test]
fn a_huge_sample_stride_is_clamped_at_the_boundary() {
    for stride in [usize::MAX, 1 << 62, MAX_SAMPLE_STEP + 1] {
        let cfg = ProtectionConfig {
            sample_step: stride,
            ..config()
        };
        assert_eq!(
            black_fraction(&black_frame(2), &cfg),
            1.0,
            "stride {stride} must not wrap the offset arithmetic"
        );
        assert!(is_black_frame(&black_frame(2), &cfg));
    }
}

#[test]
fn the_policy_debug_output_carries_no_pixels() {
    let mut policy = CapturePolicy::new(config());
    policy.observe(&black_frame(32));
    let dbg = format!("{policy:?}");
    assert!(dbg.contains("black_streak: 1"));
    assert!(!dbg.contains(CANARY));
}

#[test]
fn the_document_describes_the_fallback() {
    let doc = include_str!("../../../../docs/PROTECTED_CONTENT.md");
    assert!(doc.contains("# Protected content"));
    assert!(doc.contains("a11y_only"));
    assert!(doc.contains("SecureDesktop"));
    assert!(!doc.contains(CANARY));
}
