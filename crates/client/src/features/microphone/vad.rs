//! Адаптер общего VAD к настройкам microphone feature.

use super::backend::{MicrophoneActivationMode, MicrophoneConfig};
pub(crate) use super::core::rms_level;
use super::core::{CoreActivationMode, VoiceActivationConfig};

/// Stateful voice activation gate для основного client runtime.
#[derive(Debug, Clone)]
pub(crate) struct VoiceActivityDetector {
    detector: super::core::VoiceActivityDetector,
    config: MicrophoneConfig,
}

impl VoiceActivityDetector {
    /// Создает detector из настроек микрофона.
    pub(crate) fn new(config: MicrophoneConfig) -> Self {
        let core = VoiceActivationConfig {
            mode: match config.activation_mode {
                MicrophoneActivationMode::AlwaysActive => CoreActivationMode::AlwaysActive,
                MicrophoneActivationMode::VoiceActivated => CoreActivationMode::VoiceActivated,
            },
            threshold: config.vad_threshold,
            activation_delay_us: config.vad_activation_delay_us,
            release_delay_us: config.vad_release_delay_us,
        };
        Self {
            detector: super::core::VoiceActivityDetector::new(core),
            config,
        }
    }

    /// Обновляет detector одним level sample.
    pub(crate) fn update(&mut self, rms: f32, duration_us: u32) -> bool {
        self.detector.update(rms, duration_us)
    }

    /// Возвращает настройки микрофона.
    pub(crate) fn config(&self) -> &MicrophoneConfig {
        &self.config
    }

    /// Возвращает текущее состояние gate.
    pub(crate) fn is_active(&self) -> bool {
        self.detector.is_active()
    }
}

#[cfg(test)]
mod tests {
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
}
