use super::*;

#[test]
fn frame_size_matches_ten_milliseconds_at_supported_rates() {
    assert_eq!(frame_samples(48_000), 480);
    assert_eq!(frame_samples(24_000), 240);
    assert_eq!(frame_samples(16_000), 160);
}

#[test]
fn failed_denoising_returns_the_original_captured_pcm() {
    let mut processor = super::super::super::denoise::Processor::new();
    let original = vec![f32::NAN; 481];

    let (passed_through, failed) = captured_pcm_on_denoise_error(&mut processor, &original);

    assert!(failed);
    assert_eq!(passed_through.len(), original.len());
    assert!(
        passed_through
            .iter()
            .zip(original.iter())
            .all(|(passed, captured)| passed.to_bits() == captured.to_bits())
    );
}

#[test]
fn disabling_then_reenabling_denoising_resets_the_processor() {
    let mut processor = super::super::super::denoise::Processor::new();
    let mut was_enabled = true;
    let mut bypassed_frame = vec![0.25; 481];

    assert!(!apply_denoising(
        &mut processor,
        &mut bypassed_frame,
        false,
        &mut was_enabled,
    ));
    assert!(!was_enabled);

    let mut enabled_frame = vec![0.25; 480];
    assert!(!apply_denoising(
        &mut processor,
        &mut enabled_frame,
        true,
        &mut was_enabled,
    ));
    assert!(was_enabled);
}

#[test]
fn failed_denoising_bypasses_pcm_and_warns_once_until_disabled() {
    let mut processor = super::super::super::denoise::Processor::new();
    let mut was_enabled = false;
    let mut warning_sent = false;

    for expected_warning in [true, false] {
        let mut frame = vec![0.25; 481];
        let failed = apply_denoising(&mut processor, &mut frame, true, &mut was_enabled);

        assert!(failed);
        assert!(frame.iter().all(|sample| *sample == 0.25));
        assert_eq!(
            should_emit_denoise_warning(true, failed, &mut warning_sent),
            expected_warning
        );
    }

    assert!(!should_emit_denoise_warning(
        false,
        false,
        &mut warning_sent
    ));
    assert!(should_emit_denoise_warning(true, true, &mut warning_sent));
}

#[test]
fn non_finite_pcm_is_sanitized_before_encoding() {
    let mut frame = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.5];

    sanitize_pcm(&mut frame);

    assert_eq!(frame, [0.0, 0.0, 0.0, 0.5]);
}

#[test]
fn queued_encoded_frame_is_rejected_after_session_stops() {
    let closed = Arc::new(AtomicBool::new(false));
    let capture_closed = closed.clone();
    let frame = EncodedMicrophoneFrame {
        permission: Some(super::super::super::frame_permission::FramePermission(
            Arc::new(move || !capture_closed.load(Ordering::Acquire)),
        )),
        sequence: 0,
        timestamp_us: 0,
        duration_us: 10_000,
        codec: MicrophoneCodec::Opus,
        sample_rate_hz: 48_000,
        channels: 1,
        bytes: vec![1],
    };
    assert!(frame.can_send());

    closed.store(true, Ordering::Release);

    assert!(!frame.can_send());
}
