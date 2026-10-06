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
