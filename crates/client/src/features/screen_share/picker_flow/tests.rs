use super::{ScreenShareCaptureSource, ScreenShareTargetQuality, capture_source_from_selection};
use super::{ScreenShareControlIntent, screen_share_control_intent};
use crate::features::screen_share::ScreenShareStatus;
use crate::features::screen_share::source_picker::{
    ScreenShareFrameRate, ScreenShareResolution, ScreenShareSelection,
};

#[test]
fn selected_bitrate_is_preserved_in_capture_request() {
    let mut selection = ScreenShareSelection::browser_default();
    selection.bitrate_bps = 750_000;
    selection.resolution = ScreenShareResolution::Uhd4K;
    selection.frame_rate = ScreenShareFrameRate::Fps60;

    let ScreenShareCaptureSource::PlatformDefault {
        target: Some(target),
    } = capture_source_from_selection(&selection)
    else {
        panic!("ожидается системный выбор")
    };

    assert_eq!(target.bitrate_bps, 750_000);
    let settings = super::super::backend::ScreenShareConfig::default()
        .preset_for_target(&target)
        .expect("качество допустимо");
    let config = super::super::backend::encoder_config(settings);
    assert_eq!(
        (
            config.width,
            config.height,
            config.frame_rate,
            config.bitrate_bps
        ),
        (3840, 2160, 60, 750_000)
    );
}

#[test]
fn browser_quality_choice_keeps_platform_source_selection() {
    let selection = ScreenShareSelection {
        source_id: None,
        resolution: ScreenShareResolution::P720,
        frame_rate: ScreenShareFrameRate::Fps15,
        bitrate_bps: 1_500_000,
    };

    assert_eq!(
        capture_source_from_selection(&selection),
        ScreenShareCaptureSource::PlatformDefault {
            target: Some(ScreenShareTargetQuality {
                width: 1_280,
                height: 720,
                max_fps: 15,
                bitrate_bps: 1_500_000,
            }),
        }
    );
}

#[test]
fn opens_picker_when_platform_supports_source_selection() {
    assert_eq!(
        screen_share_control_intent(true, &ScreenShareStatus::Idle),
        ScreenShareControlIntent::OpenPicker
    );
}

#[test]
fn starts_capture_when_source_selection_is_unavailable() {
    assert_eq!(
        screen_share_control_intent(false, &ScreenShareStatus::Idle),
        ScreenShareControlIntent::Start
    );
}

#[test]
fn stops_active_capture_even_when_source_selection_is_available() {
    assert_eq!(
        screen_share_control_intent(true, &ScreenShareStatus::Live),
        ScreenShareControlIntent::Stop
    );
    assert_eq!(
        screen_share_control_intent(true, &ScreenShareStatus::Starting),
        ScreenShareControlIntent::Stop
    );
}

#[test]
fn selected_picker_choice_becomes_platform_neutral_request_source() {
    let selection = ScreenShareSelection {
        source_id: Some("\\\\.\\DISPLAY2".to_owned()),
        resolution: ScreenShareResolution::P720,
        frame_rate: ScreenShareFrameRate::Fps15,
        bitrate_bps: 1_500_000,
    };

    assert_eq!(
        capture_source_from_selection(&selection),
        ScreenShareCaptureSource::Selected {
            source_id: "\\\\.\\DISPLAY2".to_owned(),
            target: ScreenShareTargetQuality {
                width: 1_280,
                height: 720,
                max_fps: 15,
                bitrate_bps: 1_500_000,
            },
        }
    );
}
