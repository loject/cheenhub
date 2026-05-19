//! Volume-based voice activity detection.

use super::backend::{MicrophoneActivationMode, MicrophoneConfig};

const VAD_RELEASE_THRESHOLD_RATIO: f32 = 0.65;

/// Stateful voice activation gate.
#[derive(Debug, Clone)]
pub(crate) struct VoiceActivityDetector {
    config: MicrophoneConfig,
    active: bool,
    above_threshold_us: u32,
    below_threshold_us: u32,
}

impl VoiceActivityDetector {
    /// Builds a detector from microphone configuration.
    pub(crate) fn new(config: MicrophoneConfig) -> Self {
        Self {
            config,
            active: matches!(
                config.activation_mode,
                MicrophoneActivationMode::AlwaysActive
            ),
            above_threshold_us: 0,
            below_threshold_us: 0,
        }
    }

    /// Updates the detector with one level sample and returns whether audio should pass.
    pub(crate) fn update(&mut self, rms: f32, duration_us: u32) -> bool {
        match self.config.activation_mode {
            MicrophoneActivationMode::AlwaysActive => {
                self.active = true;
            }
            MicrophoneActivationMode::VoiceActivated => {
                self.update_voice_activation(rms, duration_us);
            }
        }

        self.active
    }

    /// Returns the detector configuration.
    pub(crate) fn config(&self) -> MicrophoneConfig {
        self.config
    }

    /// Returns whether the detector currently passes audio.
    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    fn update_voice_activation(&mut self, rms: f32, duration_us: u32) {
        let open_threshold = self.config.vad_threshold;
        let release_threshold = open_threshold * VAD_RELEASE_THRESHOLD_RATIO;

        if self.active {
            if rms >= release_threshold {
                self.below_threshold_us = 0;
            } else {
                self.below_threshold_us = self.below_threshold_us.saturating_add(duration_us);
                self.above_threshold_us = 0;
                if self.below_threshold_us >= self.config.vad_release_delay_us {
                    self.active = false;
                }
            }
        } else if rms >= open_threshold {
            self.above_threshold_us = self.above_threshold_us.saturating_add(duration_us);
            self.below_threshold_us = 0;
            if self.above_threshold_us >= self.config.vad_activation_delay_us {
                self.active = true;
            }
        } else {
            self.below_threshold_us = self.below_threshold_us.saturating_add(duration_us);
            self.above_threshold_us = 0;
        }
    }
}

/// Computes normalized RMS amplitude for interleaved sample values.
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
        assert!(detector.is_active());
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
