use super::{DirectCallStageStatus, stage_state_content};
use super::{ParticipantVideoSource, screen_share_card_style};

#[test]
fn screen_share_card_preserves_frame_aspect_and_fits_both_stage_dimensions() {
    for (dimensions, aspect, height_bound) in [
        ((1920, 1080), "1920 / 1080", "177.77777777777777cqh"),
        ((2520, 1080), "2520 / 1080", "233.33333333333334cqh"),
        ((1080, 1920), "1080 / 1920", "56.25cqh"),
    ] {
        let style =
            screen_share_card_style(Some(ParticipantVideoSource::ScreenShare), Some(dimensions));

        assert!(
            style.contains(&format!("aspect-ratio: {aspect};")),
            "{dimensions:?}: {style}"
        );
        assert!(
            style.contains(&format!("width: min(100cqw, {height_bound},")),
            "{dimensions:?}: {style}"
        );
        assert!(style.contains("height: auto; min-height: 0;"));
    }
}

#[test]
fn missing_or_invalid_screen_dimensions_use_widescreen_until_first_valid_frame() {
    for dimensions in [None, Some((0, 1080)), Some((1920, 0))] {
        let style = screen_share_card_style(Some(ParticipantVideoSource::ScreenShare), dimensions);

        assert!(
            style.starts_with("aspect-ratio: 16 / 9;"),
            "{dimensions:?}: {style}"
        );
    }
}

#[test]
fn camera_and_avatar_keep_the_existing_card_geometry() {
    for source in [None, Some(ParticipantVideoSource::Camera)] {
        assert!(screen_share_card_style(source, Some((2520, 1080))).is_empty());
    }
}

#[test]
fn keeps_connected_stage_free_for_peer_media() {
    assert!(stage_state_content(&DirectCallStageStatus::Connected, "Лиса").is_none());
}

#[test]
fn exposes_retry_only_for_terminal_media_error() {
    let connecting = stage_state_content(&DirectCallStageStatus::Connecting, "Лиса")
        .expect("connecting state should have explanatory content");
    let error = stage_state_content(
        &DirectCallStageStatus::Error {
            message: "Нет связи".to_owned(),
        },
        "Лиса",
    )
    .expect("error state should have explanatory content");

    assert!(!connecting.2);
    assert!(error.2);
    assert_eq!(error.1, "Нет связи");
}
