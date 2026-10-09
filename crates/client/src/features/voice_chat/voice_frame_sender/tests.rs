use super::*;
use crate::features::microphone::MicrophoneCodec;

#[test]
fn pending_voice_frame_keeps_only_latest_frame() {
    let mut pending = PendingVoiceFrame::default();

    pending.replace(frame(10));
    pending.replace(frame(11));
    pending.replace(frame(12));

    let next = pending.take().expect("ожидался последний голосовой фрейм");
    assert_eq!(next.frame.sequence, 12);
    assert_eq!(next.dropped_frames, 2);
    assert_eq!(next.first_dropped_sequence, Some(10));
    assert_eq!(next.last_dropped_sequence, Some(11));
}

#[test]
fn pending_voice_frame_resets_drop_summary_after_take() {
    let mut pending = PendingVoiceFrame::default();
    pending.replace(frame(20));
    pending.replace(frame(21));
    let _ = pending.take();

    pending.replace(frame(22));

    let next = pending.take().expect("ожидался новый голосовой фрейм");
    assert_eq!(next.frame.sequence, 22);
    assert_eq!(next.dropped_frames, 0);
    assert_eq!(next.first_dropped_sequence, None);
    assert_eq!(next.last_dropped_sequence, None);
}

fn frame(sequence: u64) -> EncodedMicrophoneFrame {
    EncodedMicrophoneFrame {
        permission: None,
        sequence,
        timestamp_us: sequence * 10_000,
        duration_us: 10_000,
        codec: MicrophoneCodec::Opus,
        sample_rate_hz: 48_000,
        channels: 1,
        bytes: vec![1, 2, 3],
    }
}
