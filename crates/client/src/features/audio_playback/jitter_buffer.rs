//! Inbound voice jitter buffering.

use std::collections::BTreeMap;

use crate::features::audio_playback::backend::VoiceFrame;

const MAX_PENDING_FRAMES: usize = 80;
const SEQUENCE_RESET_BACKWARD_THRESHOLD: u64 = 64;

/// Per-sender encoded voice jitter buffer.
#[derive(Default)]
pub(super) struct JitterBuffer {
    pending: BTreeMap<u64, QueuedVoiceFrame>,
    next_sequence: Option<u64>,
    playout_started: bool,
}

/// Result of pushing a frame into a jitter buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum JitterBufferPush {
    /// The frame was accepted into the pending queue.
    Accepted {
        /// Number of frames now waiting in the buffer.
        pending_frames: usize,
    },
    /// The frame started a new sender-local sequence and reset the old queue.
    Reset {
        /// Previously expected sequence.
        previous_expected_sequence: u64,
        /// Number of frames now waiting in the buffer.
        pending_frames: usize,
    },
    /// The frame was already present in the pending queue.
    DroppedDuplicate,
    /// The frame arrived after its playout position had already passed.
    DroppedStale {
        /// Sequence currently expected by the buffer.
        expected_sequence: u64,
    },
}

/// Frames and diagnostics released by one buffer drain.
#[derive(Default)]
pub(super) struct JitterBufferDrain {
    /// Encoded voice frames ready to decode in playout order.
    pub(super) ready_frames: Vec<VoiceFrame>,
    /// Микросекунды до следующей полезной попытки опустошить буфер.
    pub(super) next_wake_us: Option<u32>,
    /// Number of missing sequence positions skipped to keep audio moving.
    pub(super) skipped_sequences: u64,
    /// Number of stale queued frames discarded during this drain.
    pub(super) dropped_stale_frames: usize,
}

struct QueuedVoiceFrame {
    frame: VoiceFrame,
    arrival_us: u64,
}

impl JitterBuffer {
    /// Pushes one inbound frame into the reorder queue.
    pub(super) fn push(&mut self, frame: VoiceFrame, now_us: u64) -> JitterBufferPush {
        let sequence = frame.sequence;
        if let Some(expected_sequence) = self.next_sequence {
            if sequence < expected_sequence {
                if !self.playout_started {
                    self.next_sequence = Some(sequence);
                } else if self.should_reset_for(sequence, expected_sequence) {
                    self.pending.clear();
                    self.next_sequence = Some(sequence);
                    self.playout_started = false;
                    self.pending.insert(
                        sequence,
                        QueuedVoiceFrame {
                            frame,
                            arrival_us: now_us,
                        },
                    );
                    return JitterBufferPush::Reset {
                        previous_expected_sequence: expected_sequence,
                        pending_frames: self.pending.len(),
                    };
                } else {
                    return JitterBufferPush::DroppedStale { expected_sequence };
                }
            }
        } else {
            self.next_sequence = Some(sequence);
        }

        if self.pending.contains_key(&sequence) {
            return JitterBufferPush::DroppedDuplicate;
        }

        self.pending.insert(
            sequence,
            QueuedVoiceFrame {
                frame,
                arrival_us: now_us,
            },
        );

        JitterBufferPush::Accepted {
            pending_frames: self.pending.len(),
        }
    }

    /// Releases frames whose playout deadline has passed.
    pub(super) fn drain_ready(
        &mut self,
        now_us: u64,
        target_playout_delay_us: u64,
    ) -> JitterBufferDrain {
        let mut drain = JitterBufferDrain::default();

        while let Some(expected_sequence) = self.next_sequence {
            self.drop_queued_stale(expected_sequence, &mut drain);

            if let Some(queued) = self.pending.get(&expected_sequence) {
                let ready_at = queued.arrival_us.saturating_add(target_playout_delay_us);
                if ready_at > now_us {
                    drain.next_wake_us = Some(delay_until_us(ready_at, now_us));
                    break;
                }

                let queued = self
                    .pending
                    .remove(&expected_sequence)
                    .expect("expected frame exists in jitter buffer");
                drain.ready_frames.push(queued.frame);
                self.playout_started = true;
                self.next_sequence = Some(expected_sequence.saturating_add(1));
                continue;
            }

            let Some((&next_available_sequence, queued)) = self.pending.first_key_value() else {
                break;
            };
            let missing_deadline = queued.arrival_us.saturating_add(target_playout_delay_us);
            if missing_deadline > now_us && self.pending.len() < MAX_PENDING_FRAMES {
                drain.next_wake_us = Some(delay_until_us(missing_deadline, now_us));
                break;
            }

            drain.skipped_sequences = drain
                .skipped_sequences
                .saturating_add(next_available_sequence.saturating_sub(expected_sequence));
            self.playout_started = true;
            self.next_sequence = Some(next_available_sequence);
        }

        drain
    }

    /// Returns whether the buffer has no queued frames.
    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    fn should_reset_for(&self, sequence: u64, expected_sequence: u64) -> bool {
        self.pending.is_empty()
            && expected_sequence.saturating_sub(sequence) > SEQUENCE_RESET_BACKWARD_THRESHOLD
    }

    fn drop_queued_stale(&mut self, expected_sequence: u64, drain: &mut JitterBufferDrain) {
        while self
            .pending
            .first_key_value()
            .is_some_and(|(&sequence, _)| sequence < expected_sequence)
        {
            let Some(sequence) = self
                .pending
                .first_key_value()
                .map(|(&sequence, _)| sequence)
            else {
                break;
            };
            self.pending.remove(&sequence);
            drain.dropped_stale_frames = drain.dropped_stale_frames.saturating_add(1);
        }
    }
}

fn delay_until_us(deadline_us: u64, now_us: u64) -> u32 {
    deadline_us
        .saturating_sub(now_us)
        .clamp(1, u64::from(u32::MAX)) as u32
}

#[cfg(test)]
// Файл подключается дважды: как `audio_playback::jitter_buffer` из `web.rs` и как
// `cpal_playback::jitter_buffer` через `#[path]` из `native/cpal_playback.rs`. Во втором случае
// каталог дочернего модуля вычисляется относительно `native/`, поэтому путь указываем явно.
#[path = "jitter_buffer/tests.rs"]
mod tests;
