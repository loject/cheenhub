use super::{EncodedPacket, WindowsVp9Encoder, should_force_key_frame};
use crate::features::screen_share::native::windows::frames::I420Frame;

#[test]
fn first_frame_and_two_second_interval_request_key_frames() {
    assert!(should_force_key_frame(None, 0));
    assert!(!should_force_key_frame(Some(0), 1_999_999));
    assert!(should_force_key_frame(Some(0), 2_000_000));
}

#[test]
fn encodes_i420_frame_with_static_ffmpeg() {
    let mut encoder = WindowsVp9Encoder::new(128, 128, 300_000, 15).unwrap();
    let mut bytes = vec![16; 128 * 128 * 3 / 2];
    bytes[128 * 128..].fill(128);
    let frame = I420Frame { bytes };
    let mut packets = Vec::new();

    for index in 0..8 {
        packets.extend(encoder.encode(&frame, index * (1_000_000 / 15)).unwrap());
    }

    assert!(!packets.is_empty());
    assert!(packets[0].key_frame);
    assert_eq!(packets[0].duration_us, 1_000_000 / 15);
    assert_eq!(packets[0].width, 128);
    assert_eq!(packets[0].height, 128);
}

#[test]
fn resize_preserves_sequence_and_timestamps_and_starts_new_size_with_keyframe() {
    let mut encoder = WindowsVp9Encoder::new(1920, 1080, 1_500_000, 30).unwrap();
    let mut frames = Vec::new();
    let mut time = 0;
    for (width, height) in [(1920, 1080), (1280, 720), (1920, 1080)] {
        if time > 0 {
            frames.extend(encoder.finish_for_resize().unwrap());
            encoder
                .open_after_resize(width, height, 1_500_000, 30)
                .unwrap();
        }
        let frame = I420Frame {
            bytes: vec![128; width as usize * height as usize * 3 / 2],
        };
        for _ in 0..5 {
            frames.extend(encoder.encode(&frame, time).unwrap());
            time += 33_333;
        }
    }

    assert!(!frames.is_empty());
    assert!(
        frames
            .windows(2)
            .all(|pair| pair[0].sequence + 1 == pair[1].sequence
                && pair[0].timestamp_us <= pair[1].timestamp_us)
    );
    for size in [(1920, 1080), (1280, 720)] {
        let first = frames
            .iter()
            .find(|frame| (frame.width, frame.height) == size)
            .unwrap();
        assert!(first.key_frame);
    }
    let transitions: Vec<_> = frames
        .windows(2)
        .filter(|pair| (pair[0].width, pair[0].height) != (pair[1].width, pair[1].height))
        .collect();
    assert!(!transitions.is_empty());
    assert!(transitions.iter().all(|pair| pair[1].key_frame));
}

#[test]
fn failed_resize_leaves_encoder_closed() {
    let mut encoder = WindowsVp9Encoder::new(128, 128, 300_000, 15).unwrap();
    let frame = I420Frame {
        bytes: vec![128; 128 * 128 * 3 / 2],
    };
    encoder.encode(&frame, 0).unwrap();

    encoder.finish_for_resize().unwrap();
    assert!(encoder.open_after_resize(0, 720, 300_000, 15).is_err());
    assert!(encoder.encode(&frame, 1_000).is_err());
}

#[test]
fn encoder_rejects_delta_as_first_packet() {
    let mut encoder = WindowsVp9Encoder::new(128, 128, 300_000, 15).unwrap();
    let packet = EncodedPacket {
        bytes: vec![0],
        timestamp_us: 0,
        key_frame: false,
    };

    assert!(encoder.wrap_packets(vec![packet]).is_err());
}
