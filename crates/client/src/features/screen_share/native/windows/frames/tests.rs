use super::I420Frame;

#[test]
fn converts_bgra_to_i420_at_selected_size() {
    let pixels = [
        0_u8, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255,
    ];
    let frame = I420Frame::from_bgra(&pixels, 2, 2, 8, 2, 2).unwrap();
    assert_eq!(frame.bytes.len(), 6);
    assert!(frame.bytes[..4].iter().all(|value| *value > 70));
}

#[test]
fn preserves_full_source_with_black_bars() {
    let pixels = [255_u8; 64];
    let frame = I420Frame::from_bgra(&pixels, 4, 4, 16, 8, 4).unwrap();
    assert_eq!(frame.bytes.len(), 48);
    assert_eq!(&frame.bytes[..2], &[16, 16]);
    assert!(frame.bytes[2..6].iter().all(|value| *value > 200));
    assert_eq!(&frame.bytes[6..8], &[16, 16]);
}

#[test]
fn rejects_short_source_rows() {
    assert!(I420Frame::from_bgra(&[0; 8], 2, 2, 8, 2, 2).is_err());
}
