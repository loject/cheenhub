use super::*;

#[test]
fn default_frame_duration_is_twenty_milliseconds_at_supported_rates() {
    let config = MicrophoneConfig::default();
    assert_eq!(config.frame_duration_us, 20_000);
    assert_eq!(frame_samples(48_000, config.frame_duration_us), 960);
    assert_eq!(frame_samples(24_000, config.frame_duration_us), 480);
    assert_eq!(frame_samples(16_000, config.frame_duration_us), 320);
}

#[test]
fn ten_millisecond_frame_duration_remains_supported() {
    assert_eq!(frame_samples(48_000, 10_000), 480);
    assert_eq!(frame_samples(24_000, 10_000), 240);
    assert_eq!(frame_samples(16_000, 10_000), 160);
}

#[test]
fn opus_encoder_accepts_both_configured_frame_durations() {
    for frame_duration_us in [20_000, 10_000] {
        let config = MicrophoneConfig {
            frame_duration_us,
            ..MicrophoneConfig::default()
        };
        let mut encoder = create_encoder(&config).expect("Opus encoder should initialize");
        let samples = frame_samples(config.sample_rate_hz, frame_duration_us);
        let pcm = (0..samples)
            .map(|index| (index as f32 * 0.017).sin() * 0.1)
            .collect::<Vec<_>>();
        let encoded = encoder
            .encode_vec_float(&pcm, MAX_OPUS_PACKET_BYTES)
            .expect("Opus should encode the selected frame duration");
        assert!(!encoded.is_empty());
    }
}

#[test]
fn native_denoiser_accepts_twenty_millisecond_pcm_without_disabling() {
    let mut processor = super::super::super::denoise::StreamProcessor::new();
    let mut was_enabled = false;
    let mut frame = vec![0.25_f32; 960];

    let (probability, failed) = apply_denoising(&mut processor, &mut frame, true, &mut was_enabled);

    assert!(!failed, "20 ms PCM must not disable RNNoise");
    assert_eq!(
        probability.is_some(),
        super::super::super::denoise::Processor::available()
    );
    assert!(was_enabled);
    assert!(frame.iter().all(|sample| sample.is_finite()));
}

#[test]
fn failed_denoising_returns_the_original_captured_pcm() {
    let mut processor = super::super::super::denoise::StreamProcessor::new();
    let original = vec![f32::NAN; 481];

    let mut passed_through = original.clone();
    let mut was_enabled = false;

    let (probability, failed) =
        apply_denoising(&mut processor, &mut passed_through, true, &mut was_enabled);

    assert_eq!(probability, None);

    assert_eq!(failed, super::super::super::denoise::Processor::available());
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
    let mut processor = super::super::super::denoise::StreamProcessor::new();
    let mut was_enabled = true;
    let mut bypassed_frame = vec![0.25; 481];

    assert_eq!(
        apply_denoising(&mut processor, &mut bypassed_frame, false, &mut was_enabled,),
        (None, false)
    );
    assert!(!was_enabled);

    let mut enabled_frame = vec![0.25; 480];
    let (probability, failed) =
        apply_denoising(&mut processor, &mut enabled_frame, true, &mut was_enabled);
    assert!(!failed);
    assert_eq!(
        probability.is_some(),
        super::super::super::denoise::Processor::available()
    );
    assert!(was_enabled);
}

#[test]
fn failed_denoising_bypasses_pcm_and_warns_once_until_disabled() {
    let mut processor = super::super::super::denoise::StreamProcessor::new();
    let mut was_enabled = false;
    let mut warning_sent = false;

    let available = super::super::super::denoise::Processor::available();
    for expected_warning in [available, false] {
        let mut frame = vec![0.25; 481];
        let (probability, failed) =
            apply_denoising(&mut processor, &mut frame, true, &mut was_enabled);

        assert_eq!(failed, available);
        assert_eq!(probability, None);
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
        track_id: uuid::Uuid::new_v4(),
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
