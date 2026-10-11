use super::*;

#[test]
fn saved_choice_and_legacy_choice_override_platform_default() {
    for (saved, legacy, default, expected) in [
        (Some("off"), None, DenoiseMode::Rnnoise, DenoiseMode::Off),
        (
            Some("rnnoise"),
            None,
            DenoiseMode::Off,
            DenoiseMode::Rnnoise,
        ),
        (None, Some("false"), DenoiseMode::Rnnoise, DenoiseMode::Off),
        (None, Some("true"), DenoiseMode::Off, DenoiseMode::Rnnoise),
        (None, None, DenoiseMode::Rnnoise, DenoiseMode::Rnnoise),
        (None, None, DenoiseMode::Off, DenoiseMode::Off),
        (None, None, DenoiseMode::Browser, DenoiseMode::Browser),
        (
            Some("browser"),
            None,
            DenoiseMode::Off,
            DenoiseMode::Browser,
        ),
    ] {
        assert_eq!(
            DenoiseMode::from_preferences(saved, legacy, default),
            expected
        );
    }
}

#[test]
fn unavailable_saved_algorithms_are_normalized() {
    assert_eq!(
        DenoiseMode::Rnnoise.supported(false, false),
        DenoiseMode::Off
    );
    assert_eq!(
        DenoiseMode::Rnnoise.supported(false, true),
        DenoiseMode::Browser
    );
    assert_eq!(
        DenoiseMode::Browser.supported(true, false),
        DenoiseMode::Off
    );
    assert_eq!(
        DenoiseMode::Rnnoise.supported(true, false),
        DenoiseMode::Rnnoise
    );
}
