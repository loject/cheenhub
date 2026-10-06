//! Микширование decoded PCM для native output callback.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use dioxus::prelude::warn;

/// Очередь, после которой выводится warning о задержке воспроизведения.
pub(super) const SENDER_BACKLOG_WARN_SAMPLES: usize = 48_000;
const SENDER_BACKLOG_DROP_SAMPLES: usize = 96_000;

/// Уровень, ниже которого микшер пропускает сигнал без изменений.
///
/// Соответствует примерно -2 dBFS. Речь участников обычно не превышает его,
/// поэтому при громкости 100% сигнал остаётся практически нетронутым.
const SOFT_LIMIT_KNEE: f32 = 0.8;

/// Нормализованная часть мягкого ограничителя для `t` из диапазона [0.0, 1.0].
///
/// Полином `t + t² - t³` выбран потому, что удовлетворяет всем условиям на
/// границах: `f(0) = 0` и `f'(0) = 1` дают гладкое продолжение линейного
/// участка без излома, а `f(1) = 1` и `f'(1) = 0` дают плавный подход к полной
/// амплитуде без излома. Производная `f'(t) = (1 - t)(1 + 3t)` неотрицательна
/// на [0, 1], поэтому громкость никогда не убывает.
fn soft_limit_curve(t: f32) -> f32 {
    t + t * t - t * t * t
}

/// Ограничивает итоговый sample мягким насыщением вместо жёсткой обрезки.
///
/// Микшер складывает samples нескольких отправителей и умножает сумму на общую
/// громкость вывода. При настройках выше 100% сумма превышает диапазон
/// [-1.0, 1.0], который обязателен для целочисленных PCM-форматов устройства.
/// Жёсткий `clamp` срезал бы такие пики по горизонтали, давая щелчки и
/// искажения вместо усиления. Здесь сигнал плавно подводится к полной
/// амплитуде, поэтому тихие участки получают полное усиление настройки, а
/// громкие пики насыщаются без обрезки.
///
/// Контракт функции:
///
/// - `|x| <= SOFT_LIMIT_KNEE` — сигнал возвращается без изменений, поэтому
///   громкость 100% и ниже остаётся ровно такой, как задана настройкой;
/// - `|x| = 1.0` — ровно полная амплитуда, без затухания;
/// - `|x| > 1.0` — насыщение до `sign(x)`, необходимое для безопасной
///   конвертации в целочисленный PCM;
/// - нечисловые значения заменяются тишиной или знаком, чтобы один сбойный
///   sample не заглушил поток и не дал мусор в PCM.
fn soft_limit(sample: f32) -> f32 {
    let magnitude = sample.abs();
    if !magnitude.is_finite() {
        return if magnitude.is_nan() {
            0.0
        } else {
            sample.signum()
        };
    }
    if magnitude <= SOFT_LIMIT_KNEE {
        return sample;
    }
    if magnitude >= 1.0 {
        return sample.signum();
    }

    let headroom = 1.0 - SOFT_LIMIT_KNEE;
    let normalized = (magnitude - SOFT_LIMIT_KNEE) / headroom;
    sample.signum() * (SOFT_LIMIT_KNEE + headroom * soft_limit_curve(normalized))
}

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
    loop_gain: Option<f32>,
    fade_remaining_samples: Option<(usize, usize)>,
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
        // Интерполяция двух уже ограниченных samples всегда лежит в [-1.0, 1.0],
        // поэтому clamp здесь — только страховка от нечислового состояния.
        sample.clamp(-1.0, 1.0)
    }
}

impl MixerState {
    fn next_sample(&mut self) -> f32 {
        let output_gain = self.output_gain;
        let mut mixed = 0.0_f32;
        let mut finished_fades = Vec::new();
        for (sender_id, sender) in &mut self.senders {
            let is_loop_sample = sender.samples.is_empty();
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
                let gain = if is_loop_sample {
                    sender.loop_gain.unwrap_or(sender.gain)
                } else {
                    sender.gain
                };
                let fade_gain = if is_loop_sample
                    && let Some((remaining, total)) = sender.fade_remaining_samples.as_mut()
                {
                    let gain = *remaining as f32 / *total as f32;
                    *remaining = remaining.saturating_sub(1);
                    if *remaining == 0 {
                        finished_fades.push(sender_id.clone());
                    }
                    gain
                } else {
                    1.0
                };
                mixed += sample * gain * fade_gain * output_gain;
            }
        }
        for sender_id in finished_fades {
            self.senders.remove(&sender_id);
        }
        soft_limit(mixed)
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
            loop_gain: None,
            fade_remaining_samples: None,
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
    // Decoded samples already lie in [-1.0, 1.0]; the clamp is only a safety net
    // against a malformed decoder output. It must stay a clamp rather than a
    // scale, because user gain above 100% is applied later in `next_sample`.
    sender
        .samples
        .extend(samples.into_iter().map(|sample| sample.clamp(-1.0, 1.0)));
}

/// Добавляет loop после уже поставленных в очередь one-shot samples.
pub(super) fn queue_then_loop_sender_samples(
    mixer: &MixerHandle,
    sender_user_id: &str,
    one_shot_samples: Vec<f32>,
    loop_samples: Vec<f32>,
    one_shot_gain: f32,
    loop_gain: f32,
) {
    if loop_samples.is_empty() {
        return;
    }
    let Ok(mut mixer) = mixer.lock() else {
        warn!(%sender_user_id, "native audio mixer lock is poisoned; failed to queue sequential samples");
        return;
    };
    let sender = mixer.senders.entry(sender_user_id.to_owned()).or_default();
    sender.samples.clear();
    sender.samples.extend(
        one_shot_samples
            .into_iter()
            .map(|sample| sample.clamp(-1.0, 1.0)),
    );
    sender.gain = one_shot_gain;
    sender.loop_samples = Some(
        loop_samples
            .into_iter()
            .map(|sample| sample.clamp(-1.0, 1.0))
            .collect(),
    );
    sender.loop_position = 0;
    sender.loop_gain = Some(loop_gain);
    sender.fade_remaining_samples = None;
}

/// Начинает плавное затухание sender перед его удалением.
pub(super) fn fade_out_sender(mixer: &MixerHandle, sender_user_id: &str, fade_samples: usize) {
    let Ok(mut mixer) = mixer.lock() else {
        warn!(%sender_user_id, "native audio mixer lock is poisoned; failed to fade sender");
        return;
    };
    if let Some(sender) = mixer.senders.get_mut(sender_user_id) {
        let fade_samples = fade_samples.max(1);
        sender.fade_remaining_samples = Some((fade_samples, fade_samples));
    }
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
            loop_gain: None,
            fade_remaining_samples: None,
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
mod tests;
