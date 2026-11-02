use super::{
    AssessmentTone, BITRATE_MAX_SUBSCRIBED, BITRATE_MIN, MockFrameRate, MockResolution, aria_flag,
    assess, bitrate_fraction, change_for, format_bitrate, is_supported, optimize, preset_index,
    supported_pair,
};
use crate::features::screen_share::source_picker::{ScreenShareFrameRate, ScreenShareResolution};

#[test]
fn bitrate_is_printed_with_comma_and_without_trailing_zeros() {
    assert_eq!(format_bitrate(1.0), "1");
    assert_eq!(format_bitrate(0.25), "0,25");
    assert_eq!(format_bitrate(1.5), "1,5");
    assert_eq!(format_bitrate(5.0), "5");
}

#[test]
fn bitrate_fraction_spans_the_full_subscription_scale() {
    assert!((bitrate_fraction(BITRATE_MIN) - 0.0).abs() < f64::EPSILON);
    assert!((bitrate_fraction(BITRATE_MAX_SUBSCRIBED) - 1.0).abs() < f64::EPSILON);
    let free_edge = (1.0 - BITRATE_MIN) / (BITRATE_MAX_SUBSCRIBED - BITRATE_MIN);
    assert!((bitrate_fraction(1.0) - free_edge).abs() < 1e-9);
    assert!(bitrate_fraction(0.0) <= 0.0);
    assert!(bitrate_fraction(9.0) >= 1.0);
}

#[test]
fn aria_flag_maps_bool_to_string_literal() {
    assert_eq!(aria_flag(true), "true");
    assert_eq!(aria_flag(false), "false");
}

#[test]
fn preset_index_matches_exact_mock_pairs_only() {
    assert_eq!(
        preset_index(MockResolution::P720, MockFrameRate::Fps30),
        Some(0)
    );
    assert_eq!(
        preset_index(MockResolution::P1080, MockFrameRate::Fps15),
        Some(1)
    );
    assert_eq!(
        preset_index(MockResolution::P720, MockFrameRate::Fps15),
        None
    );
    assert_eq!(
        preset_index(MockResolution::Uhd4K, MockFrameRate::Fps60),
        None
    );
}

#[test]
fn supported_pair_keeps_720p_and_caps_high_resolutions() {
    assert_eq!(
        supported_pair(MockResolution::P720, MockFrameRate::Fps15),
        (ScreenShareResolution::P720, ScreenShareFrameRate::Fps15)
    );
    assert_eq!(
        supported_pair(MockResolution::P720, MockFrameRate::Fps24),
        (ScreenShareResolution::P720, ScreenShareFrameRate::Fps30)
    );
    assert_eq!(
        supported_pair(MockResolution::P720, MockFrameRate::Fps60),
        (ScreenShareResolution::P720, ScreenShareFrameRate::Fps30)
    );
    assert_eq!(
        supported_pair(MockResolution::P1080, MockFrameRate::Fps15),
        (ScreenShareResolution::P1080, ScreenShareFrameRate::Fps15)
    );
    assert_eq!(
        supported_pair(MockResolution::P1080, MockFrameRate::Fps60),
        (ScreenShareResolution::P1080, ScreenShareFrameRate::Fps15)
    );
    assert_eq!(
        supported_pair(MockResolution::Uhd4K, MockFrameRate::Fps60),
        (ScreenShareResolution::P1080, ScreenShareFrameRate::Fps15)
    );
}

#[test]
fn is_supported_accepts_only_current_model_pairs() {
    assert!(is_supported(MockResolution::P720, MockFrameRate::Fps15));
    assert!(is_supported(MockResolution::P720, MockFrameRate::Fps30));
    assert!(is_supported(MockResolution::P1080, MockFrameRate::Fps15));
    assert!(!is_supported(MockResolution::P720, MockFrameRate::Fps24));
    assert!(!is_supported(MockResolution::P720, MockFrameRate::Fps60));
    assert!(!is_supported(MockResolution::P1080, MockFrameRate::Fps30));
    assert!(!is_supported(MockResolution::P1440, MockFrameRate::Fps30));
    assert!(!is_supported(MockResolution::Uhd4K, MockFrameRate::Fps60));
}

#[test]
fn change_for_reports_mock_choice_and_supported_launch_pair() {
    let change = change_for(MockResolution::Uhd4K, MockFrameRate::Fps60);

    assert_eq!(change.mock_resolution, MockResolution::Uhd4K);
    assert_eq!(change.mock_frame_rate, MockFrameRate::Fps60);
    assert_eq!(change.resolution, ScreenShareResolution::P1080);
    assert_eq!(change.frame_rate, ScreenShareFrameRate::Fps15);
}

#[test]
fn optimize_returns_the_sharpest_supported_pair() {
    let change = optimize();

    assert_eq!(change.resolution, ScreenShareResolution::P1080);
    assert_eq!(change.frame_rate, ScreenShareFrameRate::Fps15);
}

#[test]
fn assess_marks_supported_combinations_as_good() {
    for (resolution, frame_rate) in [
        (MockResolution::P1080, MockFrameRate::Fps15),
        (MockResolution::P720, MockFrameRate::Fps30),
        (MockResolution::P720, MockFrameRate::Fps15),
    ] {
        assert_eq!(assess(resolution, frame_rate).tone, AssessmentTone::Good);
    }
}

#[test]
fn assess_marks_unsupported_combinations_as_info() {
    for (resolution, frame_rate) in [
        (MockResolution::P720, MockFrameRate::Fps24),
        (MockResolution::P720, MockFrameRate::Fps60),
        (MockResolution::P1080, MockFrameRate::Fps30),
        (MockResolution::P1440, MockFrameRate::Fps60),
        (MockResolution::Uhd4K, MockFrameRate::Fps60),
    ] {
        assert_eq!(assess(resolution, frame_rate).tone, AssessmentTone::Info);
    }
}
