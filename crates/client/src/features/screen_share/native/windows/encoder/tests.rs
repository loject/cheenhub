use super::{WindowsVp9Encoder, should_force_key_frame};
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
