//! Проверки непрерывного потока и отмены доигрывания хвоста.

use super::*;
use crate::features::audio_playback::time_scale::{VoiceTimeCompressor, catch_up_rate};

#[test]
fn regular_stream_never_compresses_or_inserts_gaps() {
    for jitter_us in [500, 10_000, 20_000, 30_000, 200_000, 400_000] {
        let target = scheduled_target_depth_seconds(jitter_us);
        let mut compressor = VoiceTimeCompressor::new(48_000);
        let mut until: Option<f64> = None;
        for frame in 0..500 {
            let now = f64::from(frame) * 0.02;
            let rate = until.map_or(1.0, |end| {
                catch_up_rate(
                    (end - now) * compressor.rate()
                        + compressor.pending_content_samples() as f64 / 48_000.0
                        - target,
                )
            });
            assert_eq!(rate, 1.0, "jitter_us={jitter_us}, frame={frame}");
            compressor.set_rate(rate);
            let output = compressor.process(&vec![0.2; 960]);
            let start = scheduled_start_seconds(now, until, jitter_us);
            if let Some(end) = until {
                assert!((start - end).abs() < 1e-9);
            }
            until = Some(start + output.len() as f64 / 48_000.0);
        }
    }
}

#[test]
fn delayed_packet_postpones_flush_without_a_second_task() {
    let pause = PlaybackPause::default();
    assert!(pause.refresh(0, 20_000, 10_000));
    let task = pause.clone();
    pause.postpone(25_000, 20_000, 10_000);
    assert!(!task.take_due(30_000));
    assert_eq!(task.remaining_us(30_000), Some(25_000));
    assert!(!pause.refresh(40_000, 20_000, 10_000));
    assert!(task.take_due(70_000));
    assert!(!task.take_due(70_000));
    assert!(pause.refresh(100_000, 20_000, 10_000));
}

#[test]
fn removed_sender_cannot_flush_a_replacement_sender() {
    let old = PlaybackPause::default();
    old.refresh(0, 20_000, 10_000);
    let waiting_task = old.clone();
    old.cancel();
    let replacement = PlaybackPause::default();
    replacement.refresh(5_000, 20_000, 10_000);
    assert_eq!(waiting_task.remaining_us(40_000), None);
    assert!(!waiting_task.take_due(40_000));
    assert!(replacement.take_due(40_000));
}

#[test]
fn pause_timeout_follows_current_frame_and_jitter_setting() {
    let pause = PlaybackPause::default();
    pause.refresh(100, 40_000, 200_000);
    assert_eq!(pause.remaining_us(100), Some(240_000));
    assert!(!pause.take_due(240_099));
    assert!(pause.take_due(240_100));
}

#[test]
fn burst_catches_up_without_recurring_gaps() {
    for jitter_us in [500, 10_000, 20_000, 30_000, 120_000, 200_000] {
        let target = scheduled_target_depth_seconds(jitter_us);
        let mut compressor = VoiceTimeCompressor::new(48_000);
        let mut until = None;
        let mut gaps = 0;
        for frame in 0..700 {
            let now = if frame < 10 {
                0.0
            } else {
                f64::from(frame - 9) * 0.02
            };
            let rate = until.map_or(1.0, |end| {
                catch_up_rate(
                    (end - now) * compressor.rate()
                        + compressor.pending_content_samples() as f64 / 48_000.0
                        - target,
                )
            });
            compressor.set_rate(rate);
            let samples = (0..960)
                .map(|sample| {
                    (2.0 * std::f32::consts::PI * 200.0 * (frame * 960 + sample) as f32 / 48_000.0)
                        .sin()
                        * 0.2
                })
                .collect::<Vec<_>>();
            let output = compressor.process(&samples);
            if output.is_empty() {
                continue;
            }
            let start = scheduled_start_seconds(now, until, jitter_us);
            if frame >= 600 && until.is_some_and(|end| start > end + 1e-9) {
                gaps += 1;
            }
            until = Some(start + output.len() as f64 / 48_000.0);
        }
        assert_eq!(gaps, 0, "jitter_us={jitter_us}");
        assert_eq!(compressor.rate(), 1.0, "jitter_us={jitter_us}");
        assert_eq!(
            compressor.pending_content_samples(),
            0,
            "jitter_us={jitter_us}"
        );
    }
}

#[test]
fn scheduler_primes_only_after_start_or_underrun() {
    assert_eq!(scheduled_start_seconds(1.0, None, 10_000), 1.02);
    assert_eq!(scheduled_start_seconds(1.0, Some(0.99), 120_000), 1.03);
    assert_eq!(scheduled_start_seconds(1.0, Some(1.0), 120_000), 1.03);
    assert_eq!(scheduled_start_seconds(1.0, Some(1.005), 10_000), 1.005);
}
