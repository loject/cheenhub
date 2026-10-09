//! Преобразование native audio samples в mono PCM.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use dioxus::prelude::warn;

/// Создаёт CPAL callback, преобразующий interleaved samples устройства в mono PCM для очереди capture.
///
/// `channels` задаёт число каналов во входном буфере; ноль обрабатывается как один канал.
/// При установленном `closed` callback перестаёт принимать данные. В режиме Push-to-talk
/// аудио принимается только при стабильном удержании: первый буфер нового удержания и буфер,
/// во время преобразования которого изменилось удержание, отбрасываются. Вне удержания
/// пустой фрагмент сбрасывает индикатор активности. Заполненная или закрытая очередь
/// отбрасывает фрагмент без ожидания; метаданные времени CPAL не используются.
pub(super) fn capture_callback<T>(
    channels: u16,
    capture: super::super::pcm::Capture,
    closed: Arc<AtomicBool>,
) -> impl FnMut(&[T], &cpal::InputCallbackInfo) + Send + 'static
where
    T: CpalInputSample,
{
    let channels = usize::from(channels.max(1));
    let mut backlog_warning_emitted = false;
    let mut previous_epoch = None;
    move |data, _info| {
        if closed.load(Ordering::Relaxed) {
            return;
        }

        let epoch = capture
            .monitor
            .as_ref()
            .and_then(|monitor| monitor.held_epoch());
        let stable = super::super::pcm::accepts_capture_epoch(
            &mut previous_epoch,
            epoch,
            capture.monitor.is_some(),
        );
        // Первый device buffer мог начаться до нажатия; ждём следующий callback.
        if epoch.is_some() && !stable {
            return;
        }
        // Пустой фрагмент закрывает индикатор активности, не сохраняя аудио вне удержания.
        let samples = if capture.monitor.is_some() && epoch.is_none() {
            Vec::new()
        } else {
            downmix_to_mono(data, channels)
        };
        if samples.is_empty() && capture.monitor.is_none() {
            return;
        }

        // Смена удержания во время downmix запрещает передачу смешанного фрагмента.
        if capture
            .monitor
            .as_ref()
            .is_some_and(|monitor| monitor.held_epoch() != epoch)
        {
            return;
        }
        match capture
            .sender
            .try_send(super::super::pcm::Chunk { samples, epoch })
        {
            Ok(()) => {
                backlog_warning_emitted = false;
            }
            Err(mpsc::TrySendError::Full(_)) => {
                if !backlog_warning_emitted {
                    backlog_warning_emitted = true;
                    warn!("native microphone input queue is backing up");
                }
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {}
        }
    }
}

/// Приводит поддерживаемые форматы CPAL samples к floating-point PCM перед downmix.
pub(super) trait CpalInputSample: Copy + Send + 'static {
    /// Преобразует sample в PCM относительно полного диапазона исходного формата.
    fn to_f32(self) -> f32;
}

impl CpalInputSample for f32 {
    fn to_f32(self) -> f32 {
        self.clamp(-1.0, 1.0)
    }
}

impl CpalInputSample for f64 {
    fn to_f32(self) -> f32 {
        (self as f32).clamp(-1.0, 1.0)
    }
}

impl CpalInputSample for i8 {
    fn to_f32(self) -> f32 {
        self as f32 / i8::MAX as f32
    }
}

impl CpalInputSample for i16 {
    fn to_f32(self) -> f32 {
        self as f32 / i16::MAX as f32
    }
}

impl CpalInputSample for i32 {
    fn to_f32(self) -> f32 {
        self as f32 / i32::MAX as f32
    }
}

impl CpalInputSample for u8 {
    fn to_f32(self) -> f32 {
        (self as f32 - 128.0) / 128.0
    }
}

impl CpalInputSample for u16 {
    fn to_f32(self) -> f32 {
        (self as f32 - 32_768.0) / 32_768.0
    }
}

impl CpalInputSample for u32 {
    fn to_f32(self) -> f32 {
        (self as f32 - 2_147_483_648.0) / 2_147_483_648.0
    }
}

fn downmix_to_mono<T: CpalInputSample>(data: &[T], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return data.iter().map(|sample| sample.to_f32()).collect();
    }

    data.chunks_exact(channels)
        .map(|frame| frame.iter().map(|sample| sample.to_f32()).sum::<f32>() / channels as f32)
        .collect()
}

#[cfg(test)]
mod tests;
