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

#[test]
fn capture_preserves_source_aspect_within_the_selected_quality() {
    for (source, limit, expected) in [
        ((3440, 1440), VideoPresetId::Screen1080p15, (1920, 804)),
        ((2520, 1080), VideoPresetId::Screen720p30, (1280, 549)),
        ((2560, 1440), VideoPresetId::Screen1080p15, (1920, 1080)),
        ((1080, 1920), VideoPresetId::Screen720p30, (405, 720)),
        ((640, 480), VideoPresetId::Screen720p30, (640, 480)),
        ((1366, 768), VideoPresetId::Screen720p30, (1280, 720)),
        ((0, 0), VideoPresetId::Screen720p30, (1, 1)),
        (
            (u32::MAX, u32::MAX),
            VideoPresetId::Screen1080p15,
            (1080, 1080),
        ),
    ] {
        let preset = limit.spec();

        let fitted = fit_capture_preset(source.0, source.1, preset);

        assert_eq!((fitted.width, fitted.height), expected, "{source:?}");
        assert_eq!(fitted.max_fps, preset.max_fps);
        assert_eq!(fitted.bitrate_bps, preset.bitrate_bps);
    }
}
