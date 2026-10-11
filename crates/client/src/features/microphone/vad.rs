//! Адаптер общего VAD к настройкам microphone feature.

use super::backend::{MicrophoneActivationMode, MicrophoneConfig};
pub(crate) use super::core::rms_level;
use super::core::{CoreActivationMode, VoiceActivationConfig};

/// Stateful voice activation gate для основного client runtime.
#[derive(Debug, Clone)]
pub(crate) struct VoiceActivityDetector {
    detector: super::core::VoiceActivityDetector,
    config: MicrophoneConfig,
    ptt_gate: super::push_to_talk::gate::Gate,
}

impl VoiceActivityDetector {
    /// Создает detector из настроек микрофона.
    pub(crate) fn new(config: MicrophoneConfig) -> Self {
        let core = VoiceActivationConfig {
            mode: match config.activation_mode {
                MicrophoneActivationMode::AlwaysActive => CoreActivationMode::AlwaysActive,
                MicrophoneActivationMode::VoiceActivated | MicrophoneActivationMode::PushToTalk => {
                    CoreActivationMode::VoiceActivated
                }
            },
            threshold: config.vad_threshold,
            neural_threshold: config.neural_vad_threshold,
            activation_delay_us: config.vad_activation_delay_us,
            release_delay_us: config.vad_release_delay_us,
        };
        Self {
            detector: super::core::VoiceActivityDetector::new(core),
            config,
            ptt_gate: super::push_to_talk::gate::Gate::default(),
        }
    }

    /// Использует вероятность речи RNNoise вместо RMS, если обработчик её предоставил.
    ///
    /// Вероятность лежит в диапазоне 0–1 и имеет независимый порог. Смена источника
    /// сбрасывает накопленные задержки, сохраняя настройки задержек и режим активации.
    pub(crate) fn update_with_speech(
        &mut self,
        rms: f32,
        speech_probability: Option<f32>,
        duration_us: u32,
        held: bool,
    ) -> bool {
        self.refresh_config();
        if self.config.activation_mode == MicrophoneActivationMode::PushToTalk {
            if held {
                self.ptt_gate.recover(false);
                self.ptt_gate.event(true);
            } else {
                self.ptt_gate.failed();
            }
            self.ptt_gate.active()
        } else {
            self.detector
                .update_with_speech(rms, speech_probability, duration_us)
        }
    }

    fn refresh_config(&mut self) {
        let Some(live) = &self.config.activation_live else {
            return;
        };
        let next = *live.lock().unwrap_or_else(|error| error.into_inner());
        if self.config.activation_mode == next.mode
            && self.config.vad_threshold == next.threshold
            && self.config.neural_vad_threshold == next.neural_threshold
            && self.config.vad_activation_delay_us == next.activation_delay_us
            && self.config.vad_release_delay_us == next.release_delay_us
        {
            return;
        }
        self.config.activation_mode = next.mode;
        self.config.vad_threshold = next.threshold;
        self.config.neural_vad_threshold = next.neural_threshold;
        self.config.vad_activation_delay_us = next.activation_delay_us;
        self.config.vad_release_delay_us = next.release_delay_us;
        self.detector.reconfigure(VoiceActivationConfig {
            mode: if next.mode == MicrophoneActivationMode::AlwaysActive {
                CoreActivationMode::AlwaysActive
            } else {
                CoreActivationMode::VoiceActivated
            },
            threshold: next.threshold,
            neural_threshold: next.neural_threshold,
            activation_delay_us: next.activation_delay_us,
            release_delay_us: next.release_delay_us,
        });
    }

    /// Возвращает источник и фактический порог текущей активации.
    pub(crate) fn activation_diagnostics(&self) -> (&'static str, f32) {
        self.detector.activation_diagnostics()
    }

    /// Возвращает текущее состояние gate.
    pub(crate) fn is_active(&self) -> bool {
        if self.config.activation_mode == MicrophoneActivationMode::PushToTalk {
            self.ptt_gate.active()
        } else {
            self.detector.is_active()
        }
    }
}

#[cfg(test)]
mod tests;
