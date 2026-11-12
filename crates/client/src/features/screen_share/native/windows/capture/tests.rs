use super::*;

#[test]
fn fitted_capture_size_reaches_windows_encoder_without_upscale() {
    let target = ScreenShareTargetQuality {
        width: 1280,
        height: 720,
        max_fps: 15,
        bitrate_bps: 300_000,
    };
    let size = fitted_size(640, 480, &target, target.bitrate_bps).unwrap();
    let pixels = vec![0; 640 * 480 * 4];
    let frame =
        I420Frame::from_bgra(&pixels, 640, 480, 640 * 4, size.0 as usize, size.1 as usize).unwrap();
    let mut encoder =
        WindowsVp9Encoder::new(size.0, size.1, target.bitrate_bps, target.max_fps).unwrap();

    let packets: Vec<_> = (0..8)
        .flat_map(|index| encoder.encode(&frame, index * 66_666).unwrap())
        .collect();

    assert!(!packets.is_empty());
    assert_eq!((packets[0].width, packets[0].height), (640, 480));
}
