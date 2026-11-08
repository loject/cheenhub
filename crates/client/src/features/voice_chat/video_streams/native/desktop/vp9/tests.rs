use super::*;

// Ключевой кадр libvpx-vp9: красное изображение 64×48, profile 0.
const KEY_FRAME: &[u8] = &[
    0x82, 0x49, 0x83, 0x42, 0x00, 0x03, 0xf0, 0x02, 0xf6, 0x00, 0x38, 0x24,
];

#[test]
fn reads_coded_and_display_dimensions_from_vp9_key_frame() {
    assert_eq!(
        dimensions(KEY_FRAME).unwrap(),
        Dimensions {
            width: 64,
            height: 48,
            display_width: 64,
            display_height: 48,
        }
    );
}

#[test]
fn rejects_truncated_and_non_key_frames() {
    for bytes in [
        &[][..],
        &[0x82][..],
        &[0x86, 0x49, 0x83, 0x42][..],
        &[0; 12][..],
    ] {
        assert!(dimensions(bytes).is_err(), "accepted {bytes:?}");
    }
}
