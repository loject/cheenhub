use super::system_audio_allowed;

#[test]
fn audio_is_allowed_only_for_confirmed_whole_screen() {
    assert!(system_audio_allowed(Some("monitor")));
    for surface in [
        None,
        Some("window"),
        Some("browser"),
        Some("unknown"),
        Some(""),
    ] {
        assert!(!system_audio_allowed(surface), "{surface:?}");
    }
}
