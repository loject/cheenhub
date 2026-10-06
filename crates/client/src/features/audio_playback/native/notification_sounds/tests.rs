use super::*;

#[test]
fn decodes_all_notification_assets() {
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
        NotificationSound::ConnectionLost,
        NotificationSound::ConnectionRestored,
        NotificationSound::ConnectionSignalLoop,
    ];

    for sound in sounds {
        let samples = notification_samples(sound).expect(sound.event_name());
        assert!(
            samples.len() > usize::try_from(AUDIO_SAMPLE_RATE_HZ / 100).unwrap(),
            "{} should produce audible PCM samples",
            sound.event_name()
        );
        assert!(
            samples.iter().any(|sample| sample.abs() > 0.01),
            "{} should not decode to silence",
            sound.event_name()
        );
    }
}

#[test]
fn adds_preroll_before_notification_samples() {
    let samples = with_preroll(vec![0.5, -0.5], 1_000);

    assert_eq!(samples.len(), NOTIFICATION_PREROLL_MS as usize + 2);
    assert!(
        samples[..NOTIFICATION_PREROLL_MS as usize]
            .iter()
            .all(|sample| *sample == 0.0)
    );
    assert_eq!(samples[NOTIFICATION_PREROLL_MS as usize], 0.5);
}
