use super::*;

fn config() -> MicrophoneConfig {
    MicrophoneConfig {
        vad_threshold: 0.2,
        vad_activation_delay_us: 40_000,
        vad_release_delay_us: 80_000,
        ..MicrophoneConfig::default()
    }
}

#[test]
fn rms_returns_zero_for_empty_samples() {
    assert_eq!(rms_level(&[]), 0.0);
}

#[test]
fn rms_detects_signal_level() {
    let level = rms_level(&[0.5, -0.5, 0.5, -0.5]);

    assert!((level - 0.5).abs() < f32::EPSILON);
}

#[test]
fn voice_activation_waits_for_activation_delay() {
    let mut detector = VoiceActivityDetector::new(config());

    assert!(!detector.update(0.3, 20_000));
    assert!(detector.update(0.3, 20_000));
}

#[test]
fn voice_activation_holds_after_level_drops() {
    let mut detector = VoiceActivityDetector::new(config());

    assert!(detector.update(0.3, 40_000));
    assert!(detector.update(0.1, 40_000));
    assert!(!detector.update(0.1, 40_000));
}

#[test]
fn voice_activation_uses_release_hysteresis() {
    let mut detector = VoiceActivityDetector::new(config());

    assert!(detector.update(0.3, 40_000));
    assert!(detector.update(0.15, 80_000));
    assert!(detector.update(0.12, 40_000));
    assert!(!detector.update(0.12, 40_000));
}

#[test]
fn always_active_keeps_gate_open() {
    let mut detector = VoiceActivityDetector::new(MicrophoneConfig {
        activation_mode: MicrophoneActivationMode::AlwaysActive,
        ..config()
    });

    assert!(detector.update(0.0, 20_000));
    assert!(detector.update(0.0, 20_000));
}

#[test]
fn push_to_talk_requires_key_and_bypasses_voice_threshold() {
    let mut detector = VoiceActivityDetector::new(MicrophoneConfig {
        activation_mode: MicrophoneActivationMode::PushToTalk,
        ..config()
    });

    assert!(!detector.update_with_key(1.0, 20_000, false));
    assert!(detector.update_with_key(0.0, 20_000, true));
    assert!(detector.is_active());
    assert!(!detector.update_with_key(1.0, 20_000, false));
    assert!(!detector.is_active());
}

#[test]
fn push_to_talk_without_global_input_keeps_gate_closed() {
    let mut detector = VoiceActivityDetector::new(MicrophoneConfig {
        activation_mode: MicrophoneActivationMode::PushToTalk,
        ..config()
    });

    assert!(!detector.update(1.0, 1_000_000));
}
