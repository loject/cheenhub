//! Runtime между входящими Opus-фреймами, jitter buffer и native PCM-микшером.

use std::time::Duration;

use dioxus::prelude::{debug, spawn, warn};
use opus::{Channels, Decoder};

use super::jitter_buffer::JitterBufferPush;
use super::mixer::{SENDER_BACKLOG_WARN_SAMPLES, queue_sender_samples, queued_sender_samples};
use super::{
    AUDIO_SAMPLE_RATE_HZ, AudioPlaybackHandle, playback_now_ms, playback_now_us,
    should_emit_sender_warning,
};
use crate::features::audio_playback::backend::{PlaybackCodec, VoiceFrame};
use crate::features::runtime::sleep_duration;

const MAX_OPUS_FRAME_SAMPLES: usize = 5_760;
const AUDIO_PLAYBACK_WARNING_INTERVAL_MS: u64 = 5_000;
const JITTER_PENDING_WARN_FRAMES: usize = 12;
const JITTER_DRAIN_WAKE_LATE_WARN_MS: u64 = 120;

impl AudioPlaybackHandle {
    /// Принимает входящий voice frame и передает его через jitter buffer.
    pub(crate) fn play_voice_frame(&self, frame: VoiceFrame) {
        if frame.codec != PlaybackCodec::Opus || frame.bytes.is_empty() {
            return;
        }
        if self.is_muted() {
            return;
        }
        if let Err(error) = self.ensure_engine() {
            warn!(
                %error,
                sender_user_id = %frame.sender_user_id,
                sequence = frame.sequence,
                "failed to initialize native audio playback"
            );
            return;
        }

        let sender_user_id = frame.sender_user_id.clone();
        let sequence = frame.sequence;
        let outcome = {
            let mut inner = self.inner.borrow_mut();
            if inner.muted {
                return;
            }
            inner
                .jitter_buffers
                .entry(sender_user_id.clone())
                .or_default()
                .push(frame, playback_now_us())
        };
        let target_delay_us = self.inner.borrow().jitter_buffer_us;

        match outcome {
            JitterBufferPush::Accepted { pending_frames } => {
                if pending_frames == 1 {
                    debug!(
                        %sender_user_id,
                        sequence,
                        target_delay_us,
                        "started native inbound voice jitter buffer"
                    );
                }
                if pending_frames >= JITTER_PENDING_WARN_FRAMES
                    && self.should_warn_jitter(&sender_user_id)
                {
                    warn!(
                        %sender_user_id,
                        sequence,
                        pending_frames,
                        target_delay_us,
                        "native inbound voice jitter buffer pending frames are backing up"
                    );
                }
            }
            JitterBufferPush::Reset {
                previous_expected_sequence,
                pending_frames,
            } => {
                debug!(
                    %sender_user_id,
                    sequence,
                    previous_expected_sequence,
                    pending_frames,
                    "reset native inbound voice jitter buffer after sender sequence restart"
                );
            }
            JitterBufferPush::DroppedDuplicate => {
                debug!(
                    %sender_user_id,
                    sequence,
                    "dropped duplicate native inbound voice frame"
                );
                return;
            }
            JitterBufferPush::DroppedStale { expected_sequence } => {
                if self.should_warn_jitter(&sender_user_id) {
                    warn!(
                        %sender_user_id,
                        sequence,
                        expected_sequence,
                        "dropped stale native inbound voice frame"
                    );
                } else {
                    debug!(
                        %sender_user_id,
                        sequence,
                        expected_sequence,
                        "dropped stale native inbound voice frame"
                    );
                }
                return;
            }
        }

        self.ensure_jitter_drain(sender_user_id);
    }

    fn ensure_jitter_drain(&self, sender_user_id: String) {
        let generation = {
            let mut inner = self.inner.borrow_mut();
            if inner.jitter_drainers.contains_key(&sender_user_id) {
                return;
            }
            inner.next_jitter_drainer_generation =
                inner.next_jitter_drainer_generation.saturating_add(1);
            let generation = inner.next_jitter_drainer_generation;
            inner
                .jitter_drainers
                .insert(sender_user_id.clone(), generation);
            generation
        };

        let handle = self.clone();
        spawn(async move {
            loop {
                let Some(next_wake_us) = handle.drain_jitter_buffer(&sender_user_id) else {
                    break;
                };
                let wake_deadline_us = playback_now_us().saturating_add(u64::from(next_wake_us));
                sleep_duration(Duration::from_micros(u64::from(next_wake_us))).await;
                let woke_at_us = playback_now_us();
                let wake_late_ms = woke_at_us.saturating_sub(wake_deadline_us) / 1_000;
                if wake_late_ms >= JITTER_DRAIN_WAKE_LATE_WARN_MS
                    && handle.should_warn_jitter(&sender_user_id)
                {
                    warn!(
                        %sender_user_id,
                        wake_late_ms,
                        scheduled_delay_us = next_wake_us,
                        "native inbound voice jitter drain woke late"
                    );
                }
            }
            handle.finish_jitter_drain(&sender_user_id, generation);
        });
    }

    fn drain_jitter_buffer(&self, sender_user_id: &str) -> Option<u32> {
        let now_us = playback_now_us();
        let (drain, target_delay_us, should_warn_jitter_advance) = {
            let mut inner = self.inner.borrow_mut();
            if inner.muted {
                inner.jitter_buffers.remove(sender_user_id);
                return None;
            }

            let target_delay_us = inner.jitter_buffer_us;
            let drain = {
                let buffer = inner.jitter_buffers.get_mut(sender_user_id)?;
                buffer.drain_ready(now_us, u64::from(target_delay_us))
            };
            let should_warn = (drain.skipped_sequences > 0 || drain.dropped_stale_frames > 0)
                && should_emit_sender_warning(
                    &mut inner.jitter_warning_at_ms,
                    sender_user_id,
                    playback_now_ms(),
                    AUDIO_PLAYBACK_WARNING_INTERVAL_MS,
                );
            (drain, target_delay_us, should_warn)
        };

        if drain.skipped_sequences > 0 || drain.dropped_stale_frames > 0 {
            if should_warn_jitter_advance {
                warn!(
                    %sender_user_id,
                    skipped_sequences = drain.skipped_sequences,
                    dropped_stale_frames = drain.dropped_stale_frames,
                    target_delay_us,
                    "advanced native inbound voice jitter buffer after network jitter"
                );
            } else {
                debug!(
                    %sender_user_id,
                    skipped_sequences = drain.skipped_sequences,
                    dropped_stale_frames = drain.dropped_stale_frames,
                    target_delay_us,
                    "advanced native inbound voice jitter buffer after network jitter"
                );
            }
        }

        let next_wake_us = drain.next_wake_us;
        for frame in drain.ready_frames {
            if let Err(error) = self.decode_voice_frame(frame) {
                warn!(
                    %error,
                    %sender_user_id,
                    "failed to decode native jitter-buffered voice frame"
                );
            }
        }

        next_wake_us
    }

    fn decode_voice_frame(&self, frame: VoiceFrame) -> Result<(), String> {
        let sender_user_id = frame.sender_user_id;
        let sequence = frame.sequence;
        let mixer = {
            let mut inner = self.inner.borrow_mut();
            let decoder = inner
                .decoders
                .entry(sender_user_id.clone())
                .or_insert_with(create_decoder);
            let mut decoded = vec![0.0_f32; MAX_OPUS_FRAME_SAMPLES];
            let samples = decoder
                .decode_float(&frame.bytes, &mut decoded, false)
                .map_err(opus_error)?;
            decoded.truncate(samples);
            let gain = inner
                .user_volumes
                .get(&sender_user_id)
                .copied()
                .unwrap_or(1.0);
            let mixer = inner
                .engine
                .as_ref()
                .map(|engine| engine.mixer.clone())
                .ok_or_else(|| "Native audio output stream не запущен.".to_owned())?;
            queue_sender_samples(&mixer, &sender_user_id, decoded, gain, sequence);
            mixer
        };
        if queued_sender_samples(&mixer, &sender_user_id) >= SENDER_BACKLOG_WARN_SAMPLES
            && self.should_warn_decoder(&sender_user_id)
        {
            warn!(
                %sender_user_id,
                sequence,
                queued_samples = queued_sender_samples(&mixer, &sender_user_id),
                "native inbound audio output queue is backing up"
            );
        }
        Ok(())
    }

    fn finish_jitter_drain(&self, sender_user_id: &str, generation: u64) {
        let mut inner = self.inner.borrow_mut();
        if inner
            .jitter_drainers
            .get(sender_user_id)
            .is_some_and(|active_generation| *active_generation == generation)
        {
            inner.jitter_drainers.remove(sender_user_id);
        }
    }

    fn should_warn_jitter(&self, sender_user_id: &str) -> bool {
        let mut inner = self.inner.borrow_mut();
        should_emit_sender_warning(
            &mut inner.jitter_warning_at_ms,
            sender_user_id,
            playback_now_ms(),
            AUDIO_PLAYBACK_WARNING_INTERVAL_MS,
        )
    }

    fn should_warn_decoder(&self, sender_user_id: &str) -> bool {
        let mut inner = self.inner.borrow_mut();
        should_emit_sender_warning(
            &mut inner.decoder_warning_at_ms,
            sender_user_id,
            playback_now_ms(),
            AUDIO_PLAYBACK_WARNING_INTERVAL_MS,
        )
    }
}

fn create_decoder() -> Decoder {
    Decoder::new(AUDIO_SAMPLE_RATE_HZ, Channels::Mono)
        .expect("native opus decoder can be created for 48 kHz mono")
}

fn opus_error(error: impl std::fmt::Display) -> String {
    format!("Native audio backend opus вернул ошибку: {error}")
}
