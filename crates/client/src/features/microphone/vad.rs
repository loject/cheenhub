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
mod tests;
