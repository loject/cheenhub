//! Микширование decoded PCM для native output callback.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use dioxus::prelude::warn;

use super::output_samples::CpalOutputSample;

/// Очередь, после которой выводится warning о задержке воспроизведения.
pub(super) const SENDER_BACKLOG_WARN_SAMPLES: usize = 48_000;
const SENDER_BACKLOG_DROP_SAMPLES: usize = 96_000;

/// Разделяемый state микшера.
pub(super) type MixerHandle = Arc<Mutex<MixerState>>;

#[derive(Default)]
pub(super) struct MixerState {
    senders: HashMap<String, SenderMixerState>,
    output_gain: f32,
}

#[derive(Default)]
struct SenderMixerState {
    samples: VecDeque<f32>,
    gain: f32,
}

/// Создает пустой микшер с общей громкостью вывода.
pub(super) fn new_mixer(output_gain: f32) -> MixerHandle {
    Arc::new(Mutex::new(MixerState {
        senders: HashMap::new(),
        output_gain,
    }))
}

/// Возвращает callback для `cpal`, который микширует mono PCM во все каналы устройства.
pub(super) fn output_callback<T>(
    channels: u16,
    source_sample_rate_hz: u32,
    output_sample_rate_hz: u32,
    mixer: MixerHandle,
) -> impl FnMut(&mut [T], &cpal::OutputCallbackInfo) + Send + 'static
where
    T: CpalOutputSample,
{
    let channels = usize::from(channels.max(1));
    let mut resampler = OutputResampler::new(source_sample_rate_hz, output_sample_rate_hz);
    move |data, _info| {
        let Ok(mut mixer) = mixer.try_lock() else {
            write_silence(data);
            return;
        };

        for frame in data.chunks_mut(channels) {
            let sample = resampler.next_sample(&mut mixer);
            for output in frame {
                *output = T::from_f32(sample);
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
            if let Some(sample) = sender.samples.pop_front() {
                mixed += sample * sender.gain * output_gain;
            }
        }
        mixed.clamp(-1.0, 1.0)
    }
}

fn write_silence<T: CpalOutputSample>(data: &mut [T]) {
    for sample in data {
        *sample = T::from_f32(0.0);
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
#[allow(dead_code)]
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
                    },
                ),
                (
                    "b".to_owned(),
                    SenderMixerState {
                        samples: VecDeque::from(vec![0.75]),
                        gain: 1.0,
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
