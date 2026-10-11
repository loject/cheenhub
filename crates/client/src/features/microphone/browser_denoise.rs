//! Browser-адаптер RNNoise для обработки PCM в основном потоке.

pub(super) mod control;

use dioxus::prelude::{debug, warn};

use super::backend::{MicrophoneCallbacks, MicrophoneConfig};
use super::denoise::StreamProcessor;

/// Хранит DSP и живую настройку одной browser capture-сессии.
pub(super) struct BrowserDenoiser {
    processor: StreamProcessor,
    config: MicrophoneConfig,
    last_enabled: bool,
}

impl BrowserDenoiser {
    /// Создаёт DSP без зависимости от worker uplink.
    pub(super) fn new(config: MicrophoneConfig) -> Self {
        Self {
            processor: StreamProcessor::new(),
            config,
            last_enabled: false,
        }
    }

    /// Обрабатывает PCM либо сохраняет его при ошибке и отключает RNNoise в текущей сессии.
    pub(super) fn process(
        &mut self,
        samples: &mut [f32],
        sample_rate_hz: u32,
        callbacks: &MicrophoneCallbacks,
    ) -> Option<f32> {
        let snapshot = self
            .config
            .denoise_enabled_live
            .as_ref()
            .map(|runtime| runtime.snapshot());
        let enabled = snapshot.map_or(self.config.denoise_enabled, |snapshot| snapshot & 1 != 0);
        if enabled != self.last_enabled {
            debug!(
                kind = "vad_source_changed",
                rnnoise = enabled,
                "browser microphone voice detection source updated"
            );
            self.last_enabled = enabled;
        }
        let result = if enabled && sample_rate_hz != 48_000 {
            Err("RNNoise requires 48 kHz PCM")
        } else {
            self.processor.process(samples, enabled)
        };
        match result {
            Ok(probability) => probability,
            Err(error) => {
                warn!(
                    kind = "denoise_failed",
                    error, "browser RNNoise failed; passing through captured PCM"
                );
                self.config.denoise_enabled = false;
                if let Some(enabled) = &self.config.denoise_enabled_live
                    && !enabled.fail(snapshot.unwrap_or(0))
                {
                    return None;
                }
                (callbacks.on_warning)("Подавление шума отключено. Микрофон продолжает работать.");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests;
