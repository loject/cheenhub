use super::*;

fn config() -> VoiceActivationConfig {
    VoiceActivationConfig {
        mode: CoreActivationMode::VoiceActivated,
        threshold: 0.2,
        activation_delay_us: 40_000,
        release_delay_us: 80_000,
    }
}

#[test]
fn voice_activation_uses_delays_and_hysteresis() {
    let mut detector = VoiceActivityDetector::new(config());
    assert!(!detector.update(0.3, 20_000));
    assert!(detector.update(0.3, 20_000));
    assert!(detector.update(0.15, 80_000));
    assert!(detector.update(0.12, 40_000));
    assert!(!detector.update(0.12, 40_000));
}

#[test]
fn pcm_helpers_apply_gain_and_measure_rms() {
    let mut samples = [0.25, -0.25];
    apply_input_gain(&mut samples, 2.0);
    assert_eq!(samples, [0.5, -0.5]);
    assert!((rms_level(&samples) - 0.5).abs() < f32::EPSILON);
    assert_eq!(duration_us(480, 48_000), 10_000);
}
