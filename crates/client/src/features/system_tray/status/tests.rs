use super::VoiceIconStatus as S;

#[test]
fn disconnected_always_has_default_icon() {
    for microphone_muted in [false, true] {
        for output_muted in [false, true] {
            assert_eq!(
                S::from_voice(false, microphone_muted, output_muted, true),
                S::Inactive
            );
        }
    }
}

#[test]
fn muted_devices_override_speaking() {
    assert_eq!(S::from_voice(true, true, false, true), S::MicrophoneMuted);
    assert_eq!(S::from_voice(true, false, true, true), S::OutputMuted);
    assert_eq!(S::from_voice(true, true, true, true), S::BothMuted);
}

#[test]
fn connected_speaking_and_idle_are_distinct() {
    assert_eq!(S::from_voice(true, false, false, false), S::Connected);
    assert_eq!(S::from_voice(true, false, false, true), S::Speaking);
}
