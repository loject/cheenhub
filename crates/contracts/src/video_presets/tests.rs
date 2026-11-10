use super::*;

#[test]
fn camera_and_screen_have_independent_bitrate_limits() {
    assert_eq!(CAMERA_BITRATE_LIMIT.source, VideoStreamSource::Camera);
    assert_eq!(CAMERA_BITRATE_LIMIT.max_bitrate_bps, 700_000);
    assert_eq!(
        SCREEN_SHARE_BITRATE_LIMIT.source,
        VideoStreamSource::ScreenShare
    );
    assert_eq!(SCREEN_SHARE_BITRATE_LIMIT.max_bitrate_bps, 1_500_000);
}
