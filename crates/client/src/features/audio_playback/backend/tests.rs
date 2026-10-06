use super::{NotificationSound, target_playout_depth_seconds};

#[test]
fn notification_multiplier_halves_regular_sounds() {
    let sounds = [
        NotificationSound::MessageReceived,
        NotificationSound::CurrentUserJoined,
        NotificationSound::CurrentUserLeft,
        NotificationSound::OtherUserJoined,
        NotificationSound::OtherUserLeft,
        NotificationSound::CameraEnabled,
        NotificationSound::CameraDisabled,
        NotificationSound::ScreenShareEnabled,
        NotificationSound::ScreenShareDisabled,
    ];

    for sound in sounds {
        assert_eq!(
            sound.volume_multiplier(),
            0.5,
            "{} should play at half volume",
            sound.event_name()
        );
    }
}

#[test]
fn notification_multiplier_halves_connection_sounds() {
    for sound in [
        NotificationSound::ConnectionLost,
        NotificationSound::ConnectionRestored,
        NotificationSound::ConnectionSignalLoop,
    ] {
        let multiplier = sound.volume_multiplier();
        assert!(
            multiplier > 0.0 && multiplier <= 0.5,
            "{} should stay quieter than half volume",
            sound.event_name()
        );
    }
}

#[test]
fn target_playout_depth_follows_jitter_buffer_setting() {
    assert!((target_playout_depth_seconds(10_000) - 0.01).abs() < 1e-9);
    assert!((target_playout_depth_seconds(200_000) - 0.2).abs() < 1e-9);
}

#[test]
fn target_playout_depth_supports_small_settings() {
    assert!((target_playout_depth_seconds(500) - 0.0005).abs() < 1e-9);
    assert_eq!(target_playout_depth_seconds(0), 0.0);
}
