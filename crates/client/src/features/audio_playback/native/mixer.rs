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
    mixer: MixerHandle,
) -> impl FnMut(&mut [T], &cpal::OutputCallbackInfo) + Send + 'static
where
    T: CpalOutputSample,
{
    let channels = usize::from(channels.max(1));
    move |data, _info| {
        let Ok(mut mixer) = mixer.try_lock() else {
            write_silence(data);
            return;
        };

        for frame in data.chunks_mut(channels) {
            let sample = mixer.next_sample();
            for output in frame {
                *output = T::from_f32(sample);
            }
        }
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
}
