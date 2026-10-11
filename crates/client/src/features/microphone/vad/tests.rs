use super::super::backend::ActivationConfig;
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

    assert!(!detector.update_with_speech(0.3, None, 20_000, false));
    assert!(detector.update_with_speech(0.3, None, 20_000, false));
}

#[test]
fn voice_activation_holds_after_level_drops() {
    let mut detector = VoiceActivityDetector::new(config());

    assert!(detector.update_with_speech(0.3, None, 40_000, false));
    assert!(detector.update_with_speech(0.1, None, 40_000, false));
    assert!(!detector.update_with_speech(0.1, None, 40_000, false));
}

#[test]
fn voice_activation_uses_release_hysteresis() {
    let mut detector = VoiceActivityDetector::new(config());

    assert!(detector.update_with_speech(0.3, None, 40_000, false));
    assert!(detector.update_with_speech(0.15, None, 80_000, false));
    assert!(detector.update_with_speech(0.12, None, 40_000, false));
    assert!(!detector.update_with_speech(0.12, None, 40_000, false));
}

#[test]
fn always_active_keeps_gate_open() {
    let mut detector = VoiceActivityDetector::new(MicrophoneConfig {
        activation_mode: MicrophoneActivationMode::AlwaysActive,
        ..config()
    });

    assert!(detector.update_with_speech(0.0, None, 20_000, false));
    assert!(detector.update_with_speech(0.0, None, 20_000, false));
}

#[test]
fn push_to_talk_requires_key_and_bypasses_voice_threshold() {
    let mut detector = VoiceActivityDetector::new(MicrophoneConfig {
        activation_mode: MicrophoneActivationMode::PushToTalk,
        ..config()
    });

    assert!(!detector.update_with_speech(1.0, None, 20_000, false));
    assert!(detector.update_with_speech(0.0, None, 20_000, true));
    assert!(detector.is_active());
    assert!(!detector.update_with_speech(1.0, None, 20_000, false));
    assert!(!detector.is_active());
}

#[test]
fn push_to_talk_without_global_input_keeps_gate_closed() {
    let mut detector = VoiceActivityDetector::new(MicrophoneConfig {
        activation_mode: MicrophoneActivationMode::PushToTalk,
        ..config()
    });

    assert!(!detector.update_with_speech(1.0, None, 1_000_000, false));
}

#[test]
fn rnnoise_rejects_loud_noise_and_accepts_quiet_speech() {
    let mut detector = VoiceActivityDetector::new(config());

    assert!(!detector.update_with_speech(1.0, Some(0.1), 40_000, false));
    assert!(detector.update_with_speech(0.0, Some(0.9), 40_000, false));
    assert!(detector.update_with_speech(1.0, Some(0.1), 40_000, false));
    assert!(!detector.update_with_speech(1.0, Some(0.1), 40_000, false));
}

#[test]
fn switching_vad_source_resets_pending_activation() {
    let mut detector = VoiceActivityDetector::new(config());

    assert!(!detector.update_with_speech(0.3, None, 20_000, false));
    assert!(!detector.update_with_speech(0.0, Some(0.9), 20_000, false));
    assert!(detector.update_with_speech(0.0, Some(0.9), 20_000, false));
    assert!(!detector.update_with_speech(0.0, None, 80_000, false));
    assert!(detector.update_with_speech(0.3, None, 40_000, false));
}

#[test]
fn rnnoise_does_not_override_push_to_talk_or_always_active() {
    let mut ptt = VoiceActivityDetector::new(MicrophoneConfig {
        activation_mode: MicrophoneActivationMode::PushToTalk,
        ..config()
    });
    let mut always = VoiceActivityDetector::new(MicrophoneConfig {
        activation_mode: MicrophoneActivationMode::AlwaysActive,
        ..config()
    });

    assert!(!ptt.update_with_speech(1.0, Some(1.0), 40_000, false));
    assert!(ptt.update_with_speech(0.0, Some(0.0), 40_000, true));
    assert!(always.update_with_speech(0.0, Some(0.0), 40_000, false));
}

#[test]
fn live_activation_change_opens_and_closes_gate_without_new_capture() {
    let initial = config();
    let live = std::sync::Arc::new(std::sync::Mutex::new(ActivationConfig {
        mode: MicrophoneActivationMode::VoiceActivated,
        threshold: initial.vad_threshold,
        neural_threshold: initial.neural_vad_threshold,
        activation_delay_us: initial.vad_activation_delay_us,
        release_delay_us: initial.vad_release_delay_us,
    }));
    let mut detector = VoiceActivityDetector::new(MicrophoneConfig {
        activation_live: Some(live.clone()),
        ..initial
    });

    assert!(!detector.update_with_speech(0.0, None, 10_000, false));
    live.lock().unwrap().mode = MicrophoneActivationMode::AlwaysActive;
    assert!(detector.update_with_speech(0.0, None, 10_000, false));
    live.lock().unwrap().mode = MicrophoneActivationMode::VoiceActivated;
    assert!(detector.update_with_speech(0.0, None, 10_000, false));
    assert!(!detector.update_with_speech(0.0, None, 80_000, false));
}
