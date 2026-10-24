use super::*;

#[test]
fn maps_microphone_status_for_notification() {
    assert_eq!(
        microphone_notification_state(MicrophoneStatus::Idle, false),
        VoiceNotificationMicrophoneState::Off
    );
    assert_eq!(
        microphone_notification_state(MicrophoneStatus::Starting, true),
        VoiceNotificationMicrophoneState::Starting
    );
    assert_eq!(
        microphone_notification_state(MicrophoneStatus::Live, true),
        VoiceNotificationMicrophoneState::Live
    );
    assert_eq!(
        microphone_notification_state(MicrophoneStatus::PermissionDenied, true),
        VoiceNotificationMicrophoneState::Unavailable
    );
}

#[test]
fn saved_mute_overrides_capture_while_stop_is_pending() {
    assert_eq!(
        microphone_notification_state(MicrophoneStatus::Live, false),
        VoiceNotificationMicrophoneState::Off
    );
    assert_eq!(
        microphone_notification_state(MicrophoneStatus::Idle, true),
        VoiceNotificationMicrophoneState::Starting
    );
}
