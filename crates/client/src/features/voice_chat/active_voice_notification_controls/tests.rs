use super::*;

#[test]
fn maps_microphone_status_for_notification() {
    assert_eq!(
        microphone_notification_state(MicrophoneStatus::Idle),
        VoiceNotificationMicrophoneState::Off
    );
    assert_eq!(
        microphone_notification_state(MicrophoneStatus::Starting),
        VoiceNotificationMicrophoneState::Starting
    );
    assert_eq!(
        microphone_notification_state(MicrophoneStatus::Live),
        VoiceNotificationMicrophoneState::Live
    );
    assert_eq!(
        microphone_notification_state(MicrophoneStatus::PermissionDenied),
        VoiceNotificationMicrophoneState::Unavailable
    );
}
