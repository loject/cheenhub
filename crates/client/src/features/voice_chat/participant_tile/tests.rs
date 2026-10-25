//! Проверки CSS-контракта пропорций декодированного видео в сфокусированном тайле.

use super::participant_video_style;

#[test]
fn decoded_dimensions_define_aspect_ratio_and_height_limited_by_stage_width() {
    for (dimensions, expected) in [
        (
            (2520, 1080),
            "--participant-video-aspect-ratio: 2520 / 1080; --participant-video-height: 42.857142857142854cqw;",
        ),
        (
            (1920, 1080),
            "--participant-video-aspect-ratio: 1920 / 1080; --participant-video-height: 56.25cqw;",
        ),
        (
            (1080, 1920),
            "--participant-video-aspect-ratio: 1080 / 1920; --participant-video-height: 177.77777777777777cqw;",
        ),
    ] {
        assert_eq!(
            participant_video_style(Some(dimensions)),
            expected,
            "{dimensions:?}"
        );
    }
}

#[test]
fn missing_or_invalid_dimensions_preserve_default_tile_geometry() {
    for dimensions in [None, Some((0, 1080)), Some((1920, 0))] {
        assert!(
            participant_video_style(dimensions).is_empty(),
            "{dimensions:?}"
        );
    }
}
