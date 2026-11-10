use super::*;

fn source(id: &str, name: &str, is_primary: bool) -> ScreenShareSource {
    ScreenShareSource {
        id: id.to_owned(),
        name: name.to_owned(),
        width: 1_920,
        height: 1_080,
        preview_url: format!("data:image/png;base64,{id}"),
        is_primary,
    }
}

#[test]
fn default_prefers_primary_source_even_when_it_is_not_first() {
    let sources = [
        source("secondary", "Экран 2", false),
        source("primary", "Основной экран", true),
    ];

    let selection = ScreenShareSelection::default_for(&sources).expect("есть экран");

    assert_eq!(selection.source_id.as_deref(), Some("primary"));
    assert_eq!(selection.resolution, ScreenShareResolution::P720);
    assert_eq!(selection.frame_rate, ScreenShareFrameRate::Fps30);
}

#[test]
fn default_uses_first_source_when_primary_is_absent() {
    let sources = [
        source("first", "Экран 1", false),
        source("second", "Экран 2", false),
    ];

    let selection = ScreenShareSelection::default_for(&sources).expect("есть экран");

    assert_eq!(selection.source_id.as_deref(), Some("first"));
}

#[test]
fn default_is_absent_without_sources() {
    assert_eq!(ScreenShareSelection::default_for(&[]), None);
}

#[test]
fn normalization_preserves_valid_selection() {
    let sources = [source("first", "Экран 1", false)];
    let current = ScreenShareSelection {
        source_id: Some("first".to_owned()),
        resolution: ScreenShareResolution::P1080,
        frame_rate: ScreenShareFrameRate::Fps15,
        bitrate_bps: 750_000,
    };

    assert_eq!(
        ScreenShareSelection::normalized_for(Some(&current), &sources),
        Some(current)
    );
}

#[test]
fn normalization_moves_quality_to_new_default_source() {
    let sources = [source("primary", "Основной экран", true)];
    let current = ScreenShareSelection {
        source_id: Some("disconnected".to_owned()),
        resolution: ScreenShareResolution::P1080,
        frame_rate: ScreenShareFrameRate::Fps15,
        bitrate_bps: 750_000,
    };

    let selection =
        ScreenShareSelection::normalized_for(Some(&current), &sources).expect("есть новый экран");

    assert_eq!(selection.source_id.as_deref(), Some("primary"));
    assert_eq!(selection.resolution, ScreenShareResolution::P1080);
    assert_eq!(selection.frame_rate, ScreenShareFrameRate::Fps15);
    assert_eq!(selection.bitrate_bps, current.bitrate_bps);
}

#[test]
fn windows_tab_never_allows_confirmation() {
    let selection = ScreenShareSelection {
        source_id: Some("primary".to_owned()),
        resolution: ScreenShareResolution::P720,
        frame_rate: ScreenShareFrameRate::Fps30,
        bitrate_bps: 1_500_000,
    };

    assert!(ScreenShareSourceTab::Screens.can_confirm(Some(&selection)));
    assert!(!ScreenShareSourceTab::Windows.can_confirm(Some(&selection)));
    assert!(!ScreenShareSourceTab::Screens.can_confirm(None));
}

#[test]
fn resolution_maps_to_exact_target_dimensions() {
    assert_eq!(ScreenShareResolution::P720.dimensions(), (1_280, 720));
    assert_eq!(ScreenShareResolution::P1080.dimensions(), (1_920, 1_080));
    assert_eq!(ScreenShareResolution::P1440.dimensions(), (2_560, 1_440));
    assert_eq!(ScreenShareResolution::Uhd4K.dimensions(), (3_840, 2_160));
}

#[test]
fn frame_rate_maps_to_exact_target_fps() {
    assert_eq!(ScreenShareFrameRate::Fps15.max_fps(), 15);
    assert_eq!(ScreenShareFrameRate::Fps24.max_fps(), 24);
    assert_eq!(ScreenShareFrameRate::Fps30.max_fps(), 30);
    assert_eq!(ScreenShareFrameRate::Fps60.max_fps(), 60);
}
