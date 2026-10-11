use super::*;

#[test]
fn initial_mode_matches_supported_desktop_or_web_build() {
    if super::super::available()
        && cfg!(any(
            all(feature = "windows", target_os = "windows"),
            all(feature = "linux", target_os = "linux")
        ))
    {
        assert_eq!(default_mode(), super::super::DenoiseMode::Rnnoise);
    } else if cfg!(feature = "web") {
        assert_eq!(default_mode(), super::super::DenoiseMode::Browser);
    } else {
        assert_eq!(default_mode(), super::super::DenoiseMode::Off);
    }
}
