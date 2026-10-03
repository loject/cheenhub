//! Доигрывание остатка native PCM после прекращения голосовых кадров.

use std::time::Duration;

use dioxus::prelude::{debug, spawn};

use super::mixer::{queue_sender_samples, queued_sender_samples};
use super::{AUDIO_SAMPLE_RATE_HZ, AudioPlaybackHandle, playback_now_us};
use crate::features::runtime::sleep_duration;

impl AudioPlaybackHandle {
    /// Продлевает ожидание паузы и запускает не более одной задачи на отправителя.
    pub(super) fn arm_playback_pause(&self, sender_user_id: &str, duration_us: u32) {
        let pause = {
            let mut inner = self.inner.borrow_mut();
            let jitter_buffer_us = inner.jitter_buffer_us;
            let pause = inner
                .playback_pauses
                .entry(sender_user_id.to_owned())
                .or_default();
            if !pause.refresh(playback_now_us(), duration_us, jitter_buffer_us) {
                return;
            }
            pause.clone()
        };
        let handle = self.clone();
        let sender_user_id = sender_user_id.to_owned();
        spawn(async move {
            while let Some(pause_wait_us) = pause.remaining_us(playback_now_us()) {
                let queued_samples = handle.inner.borrow().engine.as_ref().map_or(0, |engine| {
                    queued_sender_samples(&engine.mixer, &sender_user_id)
                });
                let queued_us = (queued_samples as u64).saturating_mul(1_000_000)
                    / u64::from(AUDIO_SAMPLE_RATE_HZ);
                let wait_us = tail_flush_wait_us(pause_wait_us, queued_us);
                if wait_us > 0 {
                    sleep_duration(Duration::from_micros(wait_us)).await;
                    continue;
                }
                pause.cancel();
                handle.flush_playback_tail(&sender_user_id, pause_wait_us, queued_samples);
                break;
            }
        });
    }

    fn flush_playback_tail(&self, sender_user_id: &str, pause_wait_us: u64, queued_samples: usize) {
        let mut inner = self.inner.borrow_mut();
        if inner.muted {
            return;
        }
        let Some(compressor) = inner.time_compressors.get_mut(sender_user_id) else {
            return;
        };
        let tail = compressor.flush();
        if tail.is_empty() {
            return;
        }
        let Some(engine) = &inner.engine else {
            return;
        };
        let gain = inner
            .user_volumes
            .get(sender_user_id)
            .copied()
            .unwrap_or(1.0);
        debug!(%sender_user_id, tail_samples = tail.len(), pause_wait_us, queued_samples,
            "flushing native voice tail before pause or PCM queue exhaustion");
        queue_sender_samples(&engine.mixer, sender_user_id, tail, gain, 0);
    }
}

/// Ограничивает ожидание хвоста оставшимся запасом PCM, в микросекундах.
///
/// Оставляет 1 мс на разрешение Tokio timer перед опустошением очереди. Пустая
/// очередь требует немедленного flush; большой запас сохраняет обычный срок паузы.
/// Запас не гарантирует своевременный вывод при задержках выполнения async-задачи.
fn tail_flush_wait_us(pause_wait_us: u64, queued_us: u64) -> u64 {
    pause_wait_us.min(queued_us.saturating_sub(1_000))
}

#[cfg(test)]
mod tests;
