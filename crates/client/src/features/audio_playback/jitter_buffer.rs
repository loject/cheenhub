//! Inbound voice jitter buffering.

use std::collections::BTreeMap;

use super::provider::VoiceFrame;

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
    /// Milliseconds until the next useful drain attempt.
    pub(super) next_wake_ms: Option<u32>,
    /// Number of missing sequence positions skipped to keep audio moving.
    pub(super) skipped_sequences: u64,
    /// Number of stale queued frames discarded during this drain.
    pub(super) dropped_stale_frames: usize,
}

struct QueuedVoiceFrame {
    frame: VoiceFrame,
    arrival_ms: u64,
}

impl JitterBuffer {
    /// Pushes one inbound frame into the reorder queue.
    pub(super) fn push(&mut self, frame: VoiceFrame, now_ms: u64) -> JitterBufferPush {
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
                            arrival_ms: now_ms,
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
                arrival_ms: now_ms,
            },
        );

        JitterBufferPush::Accepted {
            pending_frames: self.pending.len(),
        }
    }

    /// Releases frames whose playout deadline has passed.
    pub(super) fn drain_ready(
        &mut self,
        now_ms: u64,
        target_playout_delay_ms: u64,
    ) -> JitterBufferDrain {
        let mut drain = JitterBufferDrain::default();

        loop {
            let Some(expected_sequence) = self.next_sequence else {
                break;
            };

            self.drop_queued_stale(expected_sequence, &mut drain);

            if let Some(queued) = self.pending.get(&expected_sequence) {
                let ready_at = queued.arrival_ms.saturating_add(target_playout_delay_ms);
                if ready_at > now_ms {
                    drain.next_wake_ms = Some(delay_until_ms(ready_at, now_ms));
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
            let missing_deadline = queued.arrival_ms.saturating_add(target_playout_delay_ms);
            if missing_deadline > now_ms && self.pending.len() < MAX_PENDING_FRAMES {
                drain.next_wake_ms = Some(delay_until_ms(missing_deadline, now_ms));
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

fn delay_until_ms(deadline_ms: u64, now_ms: u64) -> u32 {
    deadline_ms
        .saturating_sub(now_ms)
        .clamp(1, u64::from(u32::MAX)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::audio_playback::provider::PlaybackCodec;

    const TEST_TARGET_PLAYOUT_DELAY_MS: u64 = 120;

    #[test]
    fn holds_frame_until_target_delay() {
        let mut buffer = JitterBuffer::default();

        buffer.push(frame(0), 1_000);

        let early = buffer.drain_ready(1_119, TEST_TARGET_PLAYOUT_DELAY_MS);
        assert!(early.ready_frames.is_empty());
        assert_eq!(early.next_wake_ms, Some(1));

        let ready = buffer.drain_ready(1_120, TEST_TARGET_PLAYOUT_DELAY_MS);
        assert_eq!(sequences(&ready.ready_frames), vec![0]);
        assert_eq!(ready.next_wake_ms, None);
        assert!(buffer.is_empty());
    }

    #[test]
    fn reorders_out_of_order_frames() {
        let mut buffer = JitterBuffer::default();

        buffer.push(frame(1), 1_000);
        buffer.push(frame(0), 1_020);

        let first_deadline = buffer.drain_ready(1_120, TEST_TARGET_PLAYOUT_DELAY_MS);
        assert!(first_deadline.ready_frames.is_empty());
        assert_eq!(first_deadline.next_wake_ms, Some(20));

        let ready = buffer.drain_ready(1_140, TEST_TARGET_PLAYOUT_DELAY_MS);
        assert_eq!(sequences(&ready.ready_frames), vec![0, 1]);
    }

    #[test]
    fn skips_missing_sequences_after_playout_delay() {
        let mut buffer = JitterBuffer::default();

        buffer.push(frame(0), 1_000);
        buffer.push(frame(2), 1_010);

        let first = buffer.drain_ready(1_120, TEST_TARGET_PLAYOUT_DELAY_MS);
        assert_eq!(sequences(&first.ready_frames), vec![0]);
        assert_eq!(first.next_wake_ms, Some(10));
        assert_eq!(first.skipped_sequences, 0);

        let second = buffer.drain_ready(1_130, TEST_TARGET_PLAYOUT_DELAY_MS);
        assert_eq!(sequences(&second.ready_frames), vec![2]);
        assert_eq!(second.skipped_sequences, 1);
    }

    #[test]
    fn drops_stale_frames() {
        let mut buffer = JitterBuffer::default();

        buffer.push(frame(0), 1_000);
        buffer.drain_ready(1_120, TEST_TARGET_PLAYOUT_DELAY_MS);

        assert_eq!(
            buffer.push(frame(0), 1_130),
            JitterBufferPush::DroppedStale {
                expected_sequence: 1
            }
        );
    }

    #[test]
    fn resets_after_sender_sequence_restart() {
        let mut buffer = JitterBuffer::default();

        buffer.push(frame(100), 1_000);
        buffer.drain_ready(1_120, TEST_TARGET_PLAYOUT_DELAY_MS);

        assert_eq!(
            buffer.push(frame(0), 2_000),
            JitterBufferPush::Reset {
                previous_expected_sequence: 101,
                pending_frames: 1
            }
        );

        let ready = buffer.drain_ready(2_120, TEST_TARGET_PLAYOUT_DELAY_MS);
        assert_eq!(sequences(&ready.ready_frames), vec![0]);
    }

    #[test]
    fn uses_configured_target_delay() {
        let mut buffer = JitterBuffer::default();

        buffer.push(frame(0), 1_000);

        let early = buffer.drain_ready(1_159, 160);
        assert!(early.ready_frames.is_empty());
        assert_eq!(early.next_wake_ms, Some(1));

        let ready = buffer.drain_ready(1_160, 160);
        assert_eq!(sequences(&ready.ready_frames), vec![0]);
    }

    fn frame(sequence: u64) -> VoiceFrame {
        VoiceFrame {
            sender_user_id: "sender".to_owned(),
            sequence,
            timestamp_us: sequence.saturating_mul(10_000),
            duration_us: 10_000,
            codec: PlaybackCodec::Opus,
            bytes: vec![1, 2, 3],
        }
    }

    fn sequences(frames: &[VoiceFrame]) -> Vec<u64> {
        frames.iter().map(|frame| frame.sequence).collect()
    }
}
