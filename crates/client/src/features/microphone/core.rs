//! Общая обработка PCM для browser main runtime, wasm worker и будущего native runtime.

const VAD_RELEASE_THRESHOLD_RATIO: f32 = 0.65;

/// Режим активации обработки микрофона.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CoreActivationMode {
    /// Пропускает каждый PCM chunk.
    AlwaysActive,
    /// Пропускает PCM только при открытом voice activation gate.
    VoiceActivated,
}

/// Настройки общего voice activation gate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VoiceActivationConfig {
    /// Режим активации.
    pub(crate) mode: CoreActivationMode,
    /// RMS-порог открытия gate.
    pub(crate) threshold: f32,
    /// Минимальная длительность сигнала выше порога.
    pub(crate) activation_delay_us: u32,
    /// Время удержания gate после падения сигнала.
    pub(crate) release_delay_us: u32,
}

/// Stateful voice activation gate без зависимости от UI и platform API.
#[derive(Debug, Clone)]
pub(crate) struct VoiceActivityDetector {
    config: VoiceActivationConfig,
    active: bool,
    above_threshold_us: u32,
    below_threshold_us: u32,
}

impl VoiceActivityDetector {
    /// Создает detector с указанными настройками.
    pub(crate) fn new(config: VoiceActivationConfig) -> Self {
        Self {
            active: matches!(config.mode, CoreActivationMode::AlwaysActive),
            config,
            above_threshold_us: 0,
            below_threshold_us: 0,
        }
    }

    /// Обновляет gate одним измерением уровня.
    pub(crate) fn update(&mut self, rms: f32, duration_us: u32) -> bool {
        match self.config.mode {
            CoreActivationMode::AlwaysActive => self.active = true,
            CoreActivationMode::VoiceActivated => self.update_voice_activation(rms, duration_us),
        }
        self.active
    }

    /// Возвращает текущее состояние gate.
    #[allow(dead_code)]
    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    /// Возвращает текущие настройки detector.
    #[allow(dead_code)]
    pub(crate) fn config(&self) -> VoiceActivationConfig {
        self.config
    }

    /// Обновляет состояние voice activation gate.
    fn update_voice_activation(&mut self, rms: f32, duration_us: u32) {
        let release_threshold = self.config.threshold * VAD_RELEASE_THRESHOLD_RATIO;
        if self.active {
            if rms >= release_threshold {
                self.below_threshold_us = 0;
            } else {
                self.below_threshold_us = self.below_threshold_us.saturating_add(duration_us);
                self.above_threshold_us = 0;
                if self.below_threshold_us >= self.config.release_delay_us {
                    self.active = false;
                }
            }
        } else if rms >= self.config.threshold {
            self.above_threshold_us = self.above_threshold_us.saturating_add(duration_us);
            self.below_threshold_us = 0;
            if self.above_threshold_us >= self.config.activation_delay_us {
                self.active = true;
            }
        } else {
            self.below_threshold_us = self.below_threshold_us.saturating_add(duration_us);
            self.above_threshold_us = 0;
        }
    }
}

/// Вычисляет нормализованный RMS PCM samples.
pub(crate) fn rms_level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let square_sum = samples
        .iter()
        .map(|sample| sample.clamp(-1.0, 1.0))
        .map(|sample| sample * sample)
        .sum::<f32>();
    (square_sum / samples.len() as f32).sqrt()
}

/// Применяет linear gain без выхода за PCM range.
pub(crate) fn apply_input_gain(samples: &mut [f32], input_gain: f32) {
    if (input_gain - 1.0).abs() < f32::EPSILON {
        return;
    }
    for sample in samples {
        *sample = (*sample * input_gain).clamp(-1.0, 1.0);
    }
}

/// Вычисляет длительность PCM chunk в микросекундах.
pub(crate) fn duration_us(frames: usize, sample_rate_hz: u32) -> u32 {
    ((frames as u64)
        .saturating_mul(1_000_000)
        .checked_div(u64::from(sample_rate_hz.max(1)))
        .unwrap_or(0))
    .min(u64::from(u32::MAX)) as u32
}

#[cfg(test)]
mod tests {
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
}
