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
