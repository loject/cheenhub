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
mod tests {
    use super::tail_flush_wait_us;
    use crate::features::audio_playback::playout_timing::PlaybackPause;
    use crate::features::audio_playback::time_scale::{VoiceTimeCompressor, catch_up_rate};

    #[test]
    fn native_tail_is_released_before_compressed_queue_runs_out() {
        for jitter_us in [500_u32, 10_000, 20_000, 100_000, 200_000] {
            let mut compressor = VoiceTimeCompressor::new(48_000);
            let target_samples = f64::from(jitter_us) * 0.048;
            let mut queued_samples = target_samples as usize + 4_800;
            let mut checked_tail = false;
            for _ in 0..1_000 {
                let rate = catch_up_rate(
                    (queued_samples as f64 * compressor.rate()
                        + compressor.pending_content_samples() as f64
                        - target_samples)
                        / 48_000.0,
                );
                compressor.set_rate(rate);
                queued_samples += compressor.process(&vec![0.2; 960]).len();
                let queued_us = queued_samples as u64 * 1_000_000 / 48_000;
                let pause_wait_us = u64::from(jitter_us) + 20_000;
                if rate > 1.0001
                    && compressor.pending_content_samples() > 0
                    && queued_us < pause_wait_us
                {
                    let wait_us = tail_flush_wait_us(pause_wait_us, queued_us);
                    assert!(
                        wait_us < queued_us,
                        "jitter_us={jitter_us}, wait_us={wait_us}, queued_us={queued_us}"
                    );
                    let tail = compressor.flush();
                    assert!(!tail.is_empty());
                    assert!(compressor.flush().is_empty());
                    checked_tail = true;
                    break;
                }
                queued_samples = queued_samples.saturating_sub(960);
            }
            assert!(checked_tail, "jitter_us={jitter_us}");
        }
    }

    #[test]
    fn tail_wait_preserves_pause_deadline_when_queue_is_long() {
        assert_eq!(tail_flush_wait_us(30_000, 100_000), 30_000);
        assert_eq!(tail_flush_wait_us(30_000, 0), 0);
    }

    #[test]
    fn postponed_pause_rechecks_pcm_before_waiting_again() {
        let pause = PlaybackPause::default();
        assert!(pause.refresh(0, 20_000, 10_000));
        let task = pause.clone();
        assert_eq!(
            tail_flush_wait_us(task.remaining_us(0).unwrap(), 25_000),
            24_000
        );
        pause.postpone(20_000, 20_000, 10_000);
        let remaining = task.remaining_us(24_000).unwrap();
        assert_eq!(remaining, 26_000);
        // Декодированный новый PCM продлевает ожидание той же задачи.
        assert_eq!(tail_flush_wait_us(remaining, 21_000), 20_000);
        // Пакет, ещё ожидающий в jitter buffer, не должен задерживать готовый хвост.
        assert_eq!(tail_flush_wait_us(remaining, 1_000), 0);
        pause.cancel();
        assert_eq!(task.remaining_us(24_000), None);
    }
}
