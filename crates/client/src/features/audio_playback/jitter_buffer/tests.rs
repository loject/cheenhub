use super::*;
use crate::features::audio_playback::PlaybackCodec;

const TEST_TARGET_PLAYOUT_DELAY_US: u64 = 120_000;

#[test]
fn holds_frame_until_target_delay() {
    let mut buffer = JitterBuffer::default();

    buffer.push(frame(0), 1_000_000);

    let early = buffer.drain_ready(1_119_000, TEST_TARGET_PLAYOUT_DELAY_US);
    assert!(early.ready_frames.is_empty());
    assert_eq!(early.next_wake_us, Some(1_000));

    let ready = buffer.drain_ready(1_120_000, TEST_TARGET_PLAYOUT_DELAY_US);
    assert_eq!(sequences(&ready.ready_frames), vec![0]);
    assert_eq!(ready.next_wake_us, None);
    assert!(buffer.is_empty());
}

#[test]
fn reorders_out_of_order_frames() {
    let mut buffer = JitterBuffer::default();

    buffer.push(frame(1), 1_000_000);
    buffer.push(frame(0), 1_020_000);

    let first_deadline = buffer.drain_ready(1_120_000, TEST_TARGET_PLAYOUT_DELAY_US);
    assert!(first_deadline.ready_frames.is_empty());
    assert_eq!(first_deadline.next_wake_us, Some(20_000));

    let ready = buffer.drain_ready(1_140_000, TEST_TARGET_PLAYOUT_DELAY_US);
    assert_eq!(sequences(&ready.ready_frames), vec![0, 1]);
}

#[test]
fn skips_missing_sequences_after_playout_delay() {
    let mut buffer = JitterBuffer::default();

    buffer.push(frame(0), 1_000_000);
    buffer.push(frame(2), 1_010_000);

    let first = buffer.drain_ready(1_120_000, TEST_TARGET_PLAYOUT_DELAY_US);
    assert_eq!(sequences(&first.ready_frames), vec![0]);
    assert_eq!(first.next_wake_us, Some(10_000));
    assert_eq!(first.skipped_sequences, 0);

    let second = buffer.drain_ready(1_130_000, TEST_TARGET_PLAYOUT_DELAY_US);
    assert_eq!(sequences(&second.ready_frames), vec![2]);
    assert_eq!(second.skipped_sequences, 1);
}

#[test]
fn drops_stale_frames() {
    let mut buffer = JitterBuffer::default();

    buffer.push(frame(0), 1_000_000);
    buffer.drain_ready(1_120_000, TEST_TARGET_PLAYOUT_DELAY_US);

    assert_eq!(
        buffer.push(frame(0), 1_130_000),
        JitterBufferPush::DroppedStale {
            expected_sequence: 1
        }
    );
}

#[test]
fn resets_after_sender_sequence_restart() {
    let mut buffer = JitterBuffer::default();

    buffer.push(frame(100), 1_000_000);
    buffer.drain_ready(1_120_000, TEST_TARGET_PLAYOUT_DELAY_US);

    assert_eq!(
        buffer.push(frame(0), 2_000_000),
        JitterBufferPush::Reset {
            previous_expected_sequence: 101,
            pending_frames: 1
        }
    );

    let ready = buffer.drain_ready(2_120_000, TEST_TARGET_PLAYOUT_DELAY_US);
    assert_eq!(sequences(&ready.ready_frames), vec![0]);
}

#[test]
fn uses_configured_target_delay() {
    let mut buffer = JitterBuffer::default();

    buffer.push(frame(0), 1_000_000);

    let early = buffer.drain_ready(1_159_000, 160_000);
    assert!(early.ready_frames.is_empty());
    assert_eq!(early.next_wake_us, Some(1_000));

    let ready = buffer.drain_ready(1_160_000, 160_000);
    assert_eq!(sequences(&ready.ready_frames), vec![0]);
}

#[test]
fn preserves_half_millisecond_target_delay() {
    let mut buffer = JitterBuffer::default();

    buffer.push(frame(0), 1_000_000);

    let early = buffer.drain_ready(1_000_499, 500);
    assert!(early.ready_frames.is_empty());
    assert_eq!(early.next_wake_us, Some(1));

    let ready = buffer.drain_ready(1_000_500, 500);
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
