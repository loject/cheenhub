use super::*;

#[test]
fn selects_720p_for_hd_source() {
    let spec = ScreenShareConfig::default().preset_for_capture(1366, 768);
    assert_eq!((spec.width, spec.height, spec.max_fps), (1280, 720, 30));
}

#[test]
fn selects_1080p_for_full_hd_source() {
    let spec = ScreenShareConfig::default().preset_for_capture(2560, 1440);
    assert_eq!((spec.width, spec.height, spec.max_fps), (1920, 1080, 15));
}
