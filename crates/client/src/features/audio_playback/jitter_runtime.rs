//! Runtime bridge between inbound voice frames and the jitter buffer.

use dioxus::prelude::{debug, warn};
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_futures::spawn_local;
use web_time::{SystemTime, UNIX_EPOCH};

use super::browser_helpers::js_error_message;
use super::jitter_buffer::{JitterBufferPush, TARGET_PLAYOUT_DELAY_MS};
use super::provider::{AudioPlaybackHandle, PlaybackCodec, VoiceFrame};

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

        match outcome {
            JitterBufferPush::Accepted { pending_frames } => {
                if pending_frames == 1 {
                    debug!(
                        %sender_user_id,
                        sequence,
                        target_delay_ms = TARGET_PLAYOUT_DELAY_MS,
                        "started inbound voice jitter buffer"
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
                debug!(
                    %sender_user_id,
                    sequence,
                    expected_sequence,
                    "dropped stale inbound voice frame"
                );
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
        spawn_local(async move {
            loop {
                let Some(next_wake_ms) = handle.drain_jitter_buffer(&sender_user_id) else {
                    break;
                };
                TimeoutFuture::new(next_wake_ms).await;
            }
            handle.finish_jitter_drain(&sender_user_id, generation);
        });
    }

    fn drain_jitter_buffer(&self, sender_user_id: &str) -> Option<u32> {
        let drain = {
            let mut inner = self.inner.borrow_mut();
            if inner.muted {
                inner.jitter_buffers.remove(sender_user_id);
                return None;
            }

            let buffer = inner.jitter_buffers.get_mut(sender_user_id)?;
            buffer.drain_ready(jitter_now_ms())
        };

        if drain.skipped_sequences > 0 || drain.dropped_stale_frames > 0 {
            debug!(
                %sender_user_id,
                skipped_sequences = drain.skipped_sequences,
                dropped_stale_frames = drain.dropped_stale_frames,
                "advanced inbound voice jitter buffer after network jitter"
            );
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

fn jitter_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}
