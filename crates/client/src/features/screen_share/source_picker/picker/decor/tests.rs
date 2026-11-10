use super::{
    AssessmentTone, BITRATE_MAX_SUBSCRIBED, BITRATE_MIN, ScreenShareFrameRate,
    ScreenShareResolution, aria_flag, assess, bitrate_fraction, change_for, format_bitrate,
    optimize, preset_index,
};

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
fn preset_index_matches_exact_quality_pairs() {
    assert_eq!(
        preset_index(ScreenShareResolution::P720, ScreenShareFrameRate::Fps30),
        Some(0)
    );
    assert_eq!(
        preset_index(ScreenShareResolution::P1080, ScreenShareFrameRate::Fps15),
        Some(1)
    );
    assert_eq!(
        preset_index(ScreenShareResolution::P720, ScreenShareFrameRate::Fps15),
        None
    );
    assert_eq!(
        preset_index(ScreenShareResolution::Uhd4K, ScreenShareFrameRate::Fps60),
        Some(2)
    );
}

#[test]
fn change_for_preserves_selected_quality() {
    let change = change_for(ScreenShareResolution::Uhd4K, ScreenShareFrameRate::Fps60);

    assert_eq!(change.resolution, ScreenShareResolution::Uhd4K);
    assert_eq!(change.frame_rate, ScreenShareFrameRate::Fps60);
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
        (ScreenShareResolution::P1080, ScreenShareFrameRate::Fps15),
        (ScreenShareResolution::P720, ScreenShareFrameRate::Fps30),
        (ScreenShareResolution::P720, ScreenShareFrameRate::Fps15),
    ] {
        assert_eq!(assess(resolution, frame_rate).tone, AssessmentTone::Good);
    }
}

#[test]
fn assess_marks_high_motion_and_resolution_as_info() {
    for (resolution, frame_rate) in [
        (ScreenShareResolution::P720, ScreenShareFrameRate::Fps24),
        (ScreenShareResolution::P720, ScreenShareFrameRate::Fps60),
        (ScreenShareResolution::P1080, ScreenShareFrameRate::Fps30),
        (ScreenShareResolution::P1440, ScreenShareFrameRate::Fps60),
        (ScreenShareResolution::Uhd4K, ScreenShareFrameRate::Fps60),
    ] {
        assert_eq!(assess(resolution, frame_rate).tone, AssessmentTone::Info);
    }
}
