use super::*;

#[test]
fn browser_quality_target_keeps_selected_frame_rate_and_bitrate() {
    let config = ScreenShareConfig::default();
    let target = ScreenShareTargetQuality {
        width: 1_280,
        height: 720,
        max_fps: 15,
    };

    let preset = config
        .preset_for_target(&target)
        .expect("доступный профиль");

    assert_eq!(
        (
            preset.width,
            preset.height,
            preset.max_fps,
            preset.bitrate_bps
        ),
        (1_280, 720, 15, 1_500_000)
    );
}

#[test]
fn browser_quality_target_rejects_unavailable_combination() {
    let config = ScreenShareConfig::default();
    let target = ScreenShareTargetQuality {
        width: 1_920,
        height: 1_080,
        max_fps: 30,
    };

    assert!(config.preset_for_target(&target).is_none());
}

#[test]
fn stopping_capture_revokes_an_already_queued_audio_frame() {
    let closed = Rc::new(std::cell::Cell::new(false));
    let frame = EncodedScreenAudioFrame {
        track_id: uuid::Uuid::new_v4(),
        sequence: 0,
        timestamp_us: 0,
        duration_us: 20_000,
        channels: 2,
        bytes: vec![1],
        closed: closed.clone(),
    };
    assert!(frame.can_send());

    closed.set(true);

    assert!(!frame.can_send());
}

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
