use super::*;

#[test]
fn base_presets_belong_to_expected_sources() {
    assert!(
        BASE_CAMERA_VIDEO_PRESETS
            .iter()
            .all(|preset| preset.spec().source == VideoStreamSource::Camera)
    );
    assert!(
        BASE_SCREEN_SHARE_VIDEO_PRESETS
            .iter()
            .all(|preset| preset.spec().source == VideoStreamSource::ScreenShare)
    );
}

#[test]
fn preset_ids_round_trip_through_json() {
    let encoded =
        serde_json::to_string(&VideoPresetId::Screen1080p15).expect("preset id serializes");
    let decoded: VideoPresetId = serde_json::from_str(&encoded).expect("preset id deserializes");

    assert_eq!(decoded, VideoPresetId::Screen1080p15);
}
