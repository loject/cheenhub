use super::{WindowsVp9Encoder, should_force_key_frame};
use crate::features::screen_share::native::windows::frames::I420Frame;

#[test]
fn first_frame_and_two_second_interval_request_key_frames() {
    assert!(should_force_key_frame(None, 0));
    assert!(!should_force_key_frame(Some(0), 1_999_999));
    assert!(should_force_key_frame(Some(0), 2_000_000));
}

#[test]
fn encodes_i420_frame_with_vendored_libvpx() {
    let mut encoder = WindowsVp9Encoder::new(16, 16, 300_000, 15).unwrap();
    let mut bytes = vec![16; 16 * 16 * 3 / 2];
    bytes[16 * 16..].fill(128);
    let packets = encoder.encode(&I420Frame { bytes }, 0).unwrap();
    assert!(!packets.is_empty());
    assert!(packets[0].key_frame);
    assert_eq!(packets[0].duration_us, 1_000_000 / 15);
    assert_eq!(packets[0].width, 16);
    assert_eq!(packets[0].height, 16);
}
