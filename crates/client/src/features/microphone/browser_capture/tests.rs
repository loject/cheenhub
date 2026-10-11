use super::{browser_voice_processing_enabled, browser_voice_processing_enabled_for};

#[test]
fn diagnostics_disable_browser_voice_processing() {
    assert!(!browser_voice_processing_enabled_for(true));
    assert!(browser_voice_processing_enabled_for(false));
    assert_eq!(
        browser_voice_processing_enabled(),
        !cfg!(feature = "browser-media-diagnostics")
    );
}

#[test]
fn browser_mode_requires_observed_processing_and_permission() {
    for (noise, isolation) in [
        (None, None),
        (Some(false), None),
        (Some(false), Some(false)),
    ] {
        assert!(super::verify_processing(true, true, noise, isolation).is_err());
        assert!(super::verify_processing(false, true, noise, isolation).is_ok());
    }
    for (noise, isolation) in [
        (Some(true), None),
        (None, Some(true)),
        (Some(true), Some(false)),
    ] {
        assert!(super::verify_processing(true, true, noise, isolation).is_ok());
        assert!(super::verify_processing(false, true, noise, isolation).is_err());
        assert!(super::verify_processing(true, false, noise, isolation).is_err());
    }
}
