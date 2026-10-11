use super::*;

fn config() -> VoiceActivationConfig {
    VoiceActivationConfig {
        mode: CoreActivationMode::VoiceActivated,
        threshold: 0.2,
        neural_threshold: 0.5,
        activation_delay_us: 40_000,
        release_delay_us: 80_000,
    }
}

#[test]
fn voice_activation_uses_delays_and_hysteresis() {
    let mut detector = VoiceActivityDetector::new(config());
    assert!(!detector.update_with_speech(0.3, None, 20_000));
    assert!(detector.update_with_speech(0.3, None, 20_000));
    assert!(detector.update_with_speech(0.15, None, 80_000));
    assert!(detector.update_with_speech(0.12, None, 40_000));
    assert!(!detector.update_with_speech(0.12, None, 40_000));
}

#[test]
fn pcm_helpers_apply_gain_and_measure_rms() {
    let mut samples = [0.25, -0.25];
    apply_input_gain(&mut samples, 2.0);
    assert_eq!(samples, [0.5, -0.5]);
    assert!((rms_level(&samples) - 0.5).abs() < f32::EPSILON);
    assert_eq!(duration_us(480, 48_000), 10_000);
}

#[test]
fn neural_voice_activation_ignores_volume_and_restores_rms_after_switch() {
    let mut detector = VoiceActivityDetector::new(config());

    assert!(!detector.update_with_speech(1.0, Some(0.1), 40_000));
    assert!(detector.update_with_speech(0.0, Some(0.9), 40_000));
    assert!(!detector.update_with_speech(0.0, None, 80_000));
    assert!(detector.update_with_speech(0.3, None, 40_000));
}

#[test]
fn neural_threshold_can_change_without_changing_rms_threshold() {
    let mut detector = VoiceActivityDetector::new(config());
    detector.reconfigure(VoiceActivationConfig {
        neural_threshold: 0.8,
        ..config()
    });

    assert!(!detector.update_with_speech(1.0, Some(0.7), 40_000));
    assert!(detector.update_with_speech(0.0, Some(0.9), 40_000));
    assert_eq!(detector.config().threshold, 0.2);
}

#[test]
fn parameter_changes_preserve_gate_and_accumulated_activation() {
    let mut detector = VoiceActivityDetector::new(config());
    assert!(!detector.update_with_speech(0.3, None, 20_000));

    detector.reconfigure(VoiceActivationConfig {
        threshold: 0.25,
        ..config()
    });

    assert!(detector.update_with_speech(0.3, None, 20_000));
    detector.reconfigure(VoiceActivationConfig {
        release_delay_us: 100_000,
        ..config()
    });
    assert!(detector.is_active());
}

#[test]
fn source_switch_preserves_open_gate_but_resets_source_timers() {
    let mut detector = VoiceActivityDetector::new(config());
    assert!(detector.update_with_speech(0.3, None, 40_000));

    assert!(detector.update_with_speech(0.0, Some(0.1), 40_000));
    assert!(!detector.update_with_speech(0.0, Some(0.1), 40_000));
}

#[test]
fn diagnostics_follow_neural_probability_and_rms_fallback() {
    let mut detector = VoiceActivityDetector::new(VoiceActivationConfig {
        mode: CoreActivationMode::VoiceActivated,
        threshold: 0.02,
        neural_threshold: 0.7,
        activation_delay_us: 0,
        release_delay_us: 0,
    });
    detector.update_with_speech(0.8, Some(0.6), 10_000);
    assert_eq!(detector.activation_diagnostics(), ("RNNoise", 0.7));
    detector.update_with_speech(0.8, None, 10_000);
    assert_eq!(detector.activation_diagnostics(), ("RMS", 0.02));
}
