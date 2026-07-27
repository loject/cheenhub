//! Микширование decoded PCM для native output callback.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use dioxus::prelude::warn;

/// Очередь, после которой выводится warning о задержке воспроизведения.
pub(super) const SENDER_BACKLOG_WARN_SAMPLES: usize = 48_000;
const SENDER_BACKLOG_DROP_SAMPLES: usize = 96_000;

/// Разделяемый state микшера.
pub(crate) type MixerHandle = Arc<Mutex<MixerState>>;

#[derive(Default)]
pub(crate) struct MixerState {
    senders: HashMap<String, SenderMixerState>,
    output_gain: f32,
}

#[derive(Default)]
struct SenderMixerState {
    samples: VecDeque<f32>,
    gain: f32,
    loop_samples: Option<Vec<f32>>,
    loop_position: usize,
}

/// Создает пустой микшер с общей громкостью вывода.
pub(crate) fn new_mixer(output_gain: f32) -> MixerHandle {
    Arc::new(Mutex::new(MixerState {
        senders: HashMap::new(),
        output_gain,
    }))
}

/// Состояние вывода микшера и изменения частоты PCM.
pub(crate) struct NativeOutputMixer {
    resampler: OutputResampler,
    mixer: MixerHandle,
}

impl NativeOutputMixer {
    /// Создаёт renderer для конфигурации native output stream.
    pub(crate) fn new(
        source_sample_rate_hz: u32,
        output_sample_rate_hz: u32,
        mixer: MixerHandle,
    ) -> Self {
        Self {
            resampler: OutputResampler::new(source_sample_rate_hz, output_sample_rate_hz),
            mixer,
        }
    }

    /// Рендерит указанное количество mono PCM-кадров в платформенный адаптер.
    pub(crate) fn render_frames(
        &mut self,
        frame_count: usize,
        mut write_frame: impl FnMut(usize, f32),
    ) {
        match self.mixer.try_lock() {
            Ok(mut mixer) => {
                for frame_index in 0..frame_count {
                    write_frame(frame_index, self.resampler.next_sample(&mut mixer));
                }
            }
            Err(_) => {
                for frame_index in 0..frame_count {
                    write_frame(frame_index, 0.0);
                }
            }
        }
    }
}

struct OutputResampler {
    ratio: f64,
    position: f64,
    current: f32,
    next: f32,
    initialized: bool,
}

impl OutputResampler {
    fn new(source_sample_rate_hz: u32, output_sample_rate_hz: u32) -> Self {
        let ratio = match (source_sample_rate_hz, output_sample_rate_hz) {
            (0, _) | (_, 0) => 1.0,
            (source, output) => f64::from(source) / f64::from(output),
        };

        Self {
            ratio,
            position: 0.0,
            current: 0.0,
            next: 0.0,
            initialized: false,
        }
    }

    fn next_sample(&mut self, mixer: &mut MixerState) -> f32 {
        if !self.initialized {
            self.current = mixer.next_sample();
            self.next = mixer.next_sample();
            self.initialized = true;
        }

        let sample = self.current + (self.next - self.current) * self.position as f32;
        self.position += self.ratio;
        while self.position >= 1.0 {
            self.current = self.next;
            self.next = mixer.next_sample();
            self.position -= 1.0;
        }
        sample.clamp(-1.0, 1.0)
    }
}

impl MixerState {
    fn next_sample(&mut self) -> f32 {
        let output_gain = self.output_gain;
        let mut mixed = 0.0_f32;
        for sender in self.senders.values_mut() {
            let sample = sender.samples.pop_front().or_else(|| {
                let loop_samples = sender.loop_samples.as_ref()?;
                if loop_samples.is_empty() {
                    return None;
                }
                let sample = loop_samples[sender.loop_position % loop_samples.len()];
                sender.loop_position = sender.loop_position.wrapping_add(1);
                Some(sample)
            });
            if let Some(sample) = sample {
                mixed += sample * sender.gain * output_gain;
            }
        }
        mixed.clamp(-1.0, 1.0)
    }
}

/// Добавляет decoded samples в очередь одного отправителя.
pub(super) fn queue_sender_samples(
    mixer: &MixerHandle,
    sender_user_id: &str,
    samples: Vec<f32>,
    gain: f32,
    sequence: u64,
) {
    let Ok(mut mixer) = mixer.lock() else {
        warn!(
            %sender_user_id,
            sequence,
            "native audio mixer lock is poisoned; dropped decoded frame"
        );
        return;
    };
    let sender = mixer
        .senders
        .entry(sender_user_id.to_owned())
        .or_insert_with(|| SenderMixerState {
            samples: VecDeque::new(),
            gain,
            loop_samples: None,
            loop_position: 0,
        });
    sender.gain = gain;
    if sender.samples.len() > SENDER_BACKLOG_DROP_SAMPLES {
        let drop_count = sender
            .samples
            .len()
            .saturating_sub(SENDER_BACKLOG_WARN_SAMPLES);
        sender.samples.drain(..drop_count);
        warn!(
            %sender_user_id,
            sequence,
            dropped_samples = drop_count,
            "trimmed native audio output queue backlog"
        );
    }
    sender
        .samples
        .extend(samples.into_iter().map(|sample| sample.clamp(-1.0, 1.0)));
}

/// Запускает бесконечное воспроизведение PCM-образца для одного отправителя.
pub(super) fn loop_sender_samples(
    mixer: &MixerHandle,
    sender_user_id: &str,
    samples: Vec<f32>,
    gain: f32,
) {
    if samples.is_empty() {
        return;
    }
    let Ok(mut mixer) = mixer.lock() else {
        warn!(%sender_user_id, "native audio mixer lock is poisoned; failed to loop samples");
        return;
    };
    let sender = mixer
        .senders
        .entry(sender_user_id.to_owned())
        .or_insert_with(|| SenderMixerState {
            samples: VecDeque::new(),
            gain,
            loop_samples: None,
            loop_position: 0,
        });
    sender.samples.clear();
    sender.gain = gain;
    sender.loop_samples = Some(samples);
    sender.loop_position = 0;
}

/// Возвращает количество PCM samples в очереди одного отправителя.
pub(super) fn queued_sender_samples(mixer: &MixerHandle, sender_user_id: &str) -> usize {
    mixer
        .lock()
        .ok()
        .and_then(|mixer| {
            mixer
                .senders
                .get(sender_user_id)
                .map(|sender| sender.samples.len())
        })
        .unwrap_or_default()
}

/// Обновляет индивидуальную громкость отправителя.
pub(super) fn update_sender_gain(mixer: &MixerHandle, sender_user_id: &str, gain: f32) {
    let Ok(mut mixer) = mixer.lock() else {
        warn!(
            %sender_user_id,
            "native audio mixer lock is poisoned; failed to update sender gain"
        );
        return;
    };
    mixer
        .senders
        .entry(sender_user_id.to_owned())
        .or_insert_with(|| SenderMixerState {
            samples: VecDeque::new(),
            gain,
            loop_samples: None,
            loop_position: 0,
        })
        .gain = gain;
}

/// Обновляет общую громкость вывода.
pub(super) fn update_output_gain(mixer: &MixerHandle, output_gain: f32) {
    let Ok(mut mixer) = mixer.lock() else {
        warn!("native audio mixer lock is poisoned; failed to update output gain");
        return;
    };
    mixer.output_gain = output_gain;
}

/// Удаляет queued PCM одного отправителя.
pub(super) fn remove_sender(mixer: &MixerHandle, sender_user_id: &str) {
    let Ok(mut mixer) = mixer.lock() else {
        warn!(
            %sender_user_id,
            "native audio mixer lock is poisoned; failed to remove sender"
        );
        return;
    };
    mixer.senders.remove(sender_user_id);
}

/// Очищает все queued PCM.
pub(super) fn clear_mixer(mixer: &MixerHandle) {
    let Ok(mut mixer) = mixer.lock() else {
        warn!("native audio mixer lock is poisoned; failed to clear playback");
        return;
    };
    mixer.senders.clear();
}

/// Очищает голосовые очереди, оставляя системные notification-звуки.
pub(super) fn clear_voice_senders(mixer: &MixerHandle) {
    let Ok(mut mixer) = mixer.lock() else {
        warn!("native audio mixer lock is poisoned; failed to clear voice playback");
        return;
    };
    mixer
        .senders
        .retain(|sender_id, _| sender_id.starts_with("notification:"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixer_sums_senders_and_clamps_output() {
        let mut mixer = MixerState {
            senders: HashMap::from([
                (
                    "a".to_owned(),
                    SenderMixerState {
                        samples: VecDeque::from(vec![0.75]),
                        gain: 1.0,
                        loop_samples: None,
                        loop_position: 0,
                    },
                ),
                (
                    "b".to_owned(),
                    SenderMixerState {
                        samples: VecDeque::from(vec![0.75]),
                        gain: 1.0,
                        loop_samples: None,
                        loop_position: 0,
                    },
                ),
            ]),
            output_gain: 1.0,
        };

        assert_eq!(mixer.next_sample(), 1.0);
    }

    #[test]
    fn resampler_preserves_source_samples_at_equal_rate() {
        let mixer = new_mixer(1.0);
        queue_sender_samples(&mixer, "sender", vec![0.25, 0.5, 0.75], 1.0, 1);
        let mut mixer = mixer.lock().expect("mixer lock");
        let mut resampler = OutputResampler::new(48_000, 48_000);

        assert_eq!(resampler.next_sample(&mut mixer), 0.25);
        assert_eq!(resampler.next_sample(&mut mixer), 0.5);
        assert_eq!(resampler.next_sample(&mut mixer), 0.75);
    }

    #[test]
    fn looped_sender_restarts_after_last_sample() {
        let mixer = new_mixer(1.0);
        loop_sender_samples(&mixer, "signal", vec![0.25, 0.5], 1.0);
        let mut mixer = mixer.lock().expect("mixer lock");

        assert_eq!(mixer.next_sample(), 0.25);
        assert_eq!(mixer.next_sample(), 0.5);
        assert_eq!(mixer.next_sample(), 0.25);
    }

    #[test]
    fn resampler_interpolates_when_output_rate_is_higher() {
        let mixer = new_mixer(1.0);
        queue_sender_samples(&mixer, "sender", vec![0.0, 1.0], 1.0, 1);
        let mut mixer = mixer.lock().expect("mixer lock");
        let mut resampler = OutputResampler::new(48_000, 96_000);

        assert_eq!(resampler.next_sample(&mut mixer), 0.0);
        assert_eq!(resampler.next_sample(&mut mixer), 0.5);
        assert_eq!(resampler.next_sample(&mut mixer), 1.0);
    }
}
