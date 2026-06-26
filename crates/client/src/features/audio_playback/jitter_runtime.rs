//! Runtime bridge between inbound voice frames and the jitter buffer.

use dioxus::prelude::{debug, warn};
use web_time::{SystemTime, UNIX_EPOCH};

use crate::features::runtime::sleep_ms;

use super::browser_helpers::js_error_message;
use super::jitter_buffer::JitterBufferPush;
use super::{
    AUDIO_PLAYBACK_WARNING_INTERVAL_MS, AudioPlaybackHandle, AudioPlaybackInner,
    audio_playback_now_ms, should_emit_sender_warning,
};
use crate::features::audio_playback::backend::{PlaybackCodec, VoiceFrame};

const JITTER_PENDING_WARN_FRAMES: usize = 12;
const JITTER_DRAIN_WAKE_LATE_WARN_MS: u64 = 120;

impl AudioPlaybackHandle {
    /// Plays one encoded voice frame.
    pub(crate) fn play_voice_frame(&self, frame: VoiceFrame) {
        if frame.codec != PlaybackCodec::Opus || frame.bytes.is_empty() {
            return;
        }
        if self.is_muted() {
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
                .push(frame, jitter_now_ms())
        };
        let target_delay_ms = self.inner.borrow().jitter_buffer_ms;

        match outcome {
            JitterBufferPush::Accepted { pending_frames } => {
                if pending_frames == 1 {
                    debug!(
                        %sender_user_id,
                        sequence,
                        target_delay_ms,
                        "started inbound voice jitter buffer"
                    );
                }
                if pending_frames >= JITTER_PENDING_WARN_FRAMES {
                    let should_warn = {
                        let mut inner = self.inner.borrow_mut();
                        should_warn_jitter(&mut inner, &sender_user_id, audio_playback_now_ms())
                    };
                    if should_warn {
                        warn!(
                            %sender_user_id,
                            sequence,
                            pending_frames,
                            target_delay_ms,
                            "inbound voice jitter buffer pending frames are backing up"
                        );
                    }
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
                    "reset inbound voice jitter buffer after sender sequence restart"
                );
            }
            JitterBufferPush::DroppedDuplicate => {
                debug!(
                    %sender_user_id,
                    sequence,
                    "dropped duplicate inbound voice frame"
                );
                return;
            }
            JitterBufferPush::DroppedStale { expected_sequence } => {
                let should_warn = {
                    let mut inner = self.inner.borrow_mut();
                    should_warn_jitter(&mut inner, &sender_user_id, audio_playback_now_ms())
                };
                if should_warn {
                    warn!(
                        %sender_user_id,
                        sequence,
                        expected_sequence,
                        "dropped stale inbound voice frame"
                    );
                } else {
                    debug!(
                        %sender_user_id,
                        sequence,
                        expected_sequence,
                        "dropped stale inbound voice frame"
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
        dioxus::prelude::spawn(async move {
            loop {
                let Some(next_wake_ms) = handle.drain_jitter_buffer(&sender_user_id) else {
                    break;
                };
                let wake_deadline_ms = jitter_now_ms().saturating_add(u64::from(next_wake_ms));
                sleep_ms(next_wake_ms).await;
                let woke_at_ms = jitter_now_ms();
                let wake_late_ms = woke_at_ms.saturating_sub(wake_deadline_ms);
                if wake_late_ms >= JITTER_DRAIN_WAKE_LATE_WARN_MS {
                    let should_warn = {
                        let mut inner = handle.inner.borrow_mut();
                        should_warn_jitter(&mut inner, &sender_user_id, woke_at_ms)
                    };
                    if should_warn {
                        warn!(
                            %sender_user_id,
                            wake_late_ms,
                            scheduled_delay_ms = next_wake_ms,
                            "inbound voice jitter drain woke late"
                        );
                    }
                }
            }
            handle.finish_jitter_drain(&sender_user_id, generation);
        });
    }

    fn drain_jitter_buffer(&self, sender_user_id: &str) -> Option<u32> {
        let now_ms = jitter_now_ms();
        let (drain, target_delay_ms, should_warn_jitter_advance) = {
            let mut inner = self.inner.borrow_mut();
            if inner.muted {
                inner.jitter_buffers.remove(sender_user_id);
                return None;
            }

            let target_delay_ms = inner.jitter_buffer_ms;
            let drain = {
                let buffer = inner.jitter_buffers.get_mut(sender_user_id)?;
                buffer.drain_ready(now_ms, u64::from(target_delay_ms))
            };
            let should_warn = (drain.skipped_sequences > 0 || drain.dropped_stale_frames > 0)
                && should_warn_jitter(&mut inner, sender_user_id, now_ms);
            (drain, target_delay_ms, should_warn)
        };

        if drain.skipped_sequences > 0 || drain.dropped_stale_frames > 0 {
            if should_warn_jitter_advance {
                warn!(
                    %sender_user_id,
                    skipped_sequences = drain.skipped_sequences,
                    dropped_stale_frames = drain.dropped_stale_frames,
                    target_delay_ms,
                    "advanced inbound voice jitter buffer after network jitter"
                );
            } else {
                debug!(
                    %sender_user_id,
                    skipped_sequences = drain.skipped_sequences,
                    dropped_stale_frames = drain.dropped_stale_frames,
                    target_delay_ms,
                    "advanced inbound voice jitter buffer after network jitter"
                );
            }
        }

        let next_wake_ms = drain.next_wake_ms;
        for frame in drain.ready_frames {
            if let Err(error) = self.decode_voice_frame(frame) {
                warn!(
                    error = %js_error_message(error),
                    %sender_user_id,
                    "failed to decode jitter-buffered voice frame"
                );
            }
        }

        next_wake_ms
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
}

fn should_warn_jitter(inner: &mut AudioPlaybackInner, sender_user_id: &str, now_ms: u64) -> bool {
    should_emit_sender_warning(
        &mut inner.jitter_warning_at_ms,
        sender_user_id,
        now_ms,
        AUDIO_PLAYBACK_WARNING_INTERVAL_MS,
    )
}

fn jitter_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}
