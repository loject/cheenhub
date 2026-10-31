use super::*;
use crate::features::audio_playback::backend::PlaybackCodec;

#[test]
fn microphone_and_system_tracks_keep_separate_playback_state() {
    let mut tracks = PlaybackTracks::default();
    let microphone = frame(MediaAudioSource::Microphone);
    let system = frame(MediaAudioSource::System);

    let mic = tracks.admit(&microphone).unwrap();
    let sound = tracks.admit(&system).unwrap();

    assert_ne!(mic.key, sound.key);
    assert!(mic.replaced.is_none());
    assert!(sound.replaced.is_none());
    assert_eq!(tracks.admit(&microphone).unwrap().key, mic.key);
}

#[test]
fn simultaneous_microphone_sessions_share_account_without_retiring_each_other() {
    let mut tracks = PlaybackTracks::default();
    let mut first = frame(MediaAudioSource::Microphone);
    let mut second = frame(MediaAudioSource::Microphone);
    first.session_track = true;
    second.session_track = true;

    let first_key = tracks.admit(&first).unwrap().key;
    let second_admission = tracks.admit(&second).unwrap();

    assert_ne!(first_key, second_admission.key);
    assert!(second_admission.replaced.is_none());
    assert_eq!(tracks.admit(&first).unwrap().key, first_key);
    assert_eq!(tracks.admit(&second).unwrap().key, second_admission.key);
}

#[test]
fn confirmed_microphone_session_recovers_after_provisional_legacy_replacement() {
    let mut tracks = PlaybackTracks::default();
    let mut first = frame(MediaAudioSource::Microphone);
    let second = frame(MediaAudioSource::Microphone);
    let first_key = tracks.admit(&first).unwrap().key;

    assert_eq!(
        tracks.admit(&second).unwrap().replaced,
        Some(first_key.clone())
    );
    first.session_track = true;

    assert_eq!(tracks.admit(&first).unwrap().key, first_key);
}

#[test]
fn stopping_unconfirmed_session_releases_legacy_decoder_and_blocks_late_frames() {
    let mut tracks = PlaybackTracks::default();
    let mut microphone = frame(MediaAudioSource::Microphone);
    let key = tracks.admit(&microphone).unwrap().key;

    assert_eq!(
        tracks.stop_microphone_session("sender", microphone.track_id),
        Some(key)
    );
    microphone.session_track = true;
    assert!(tracks.admit(&microphone).is_none());
}

#[test]
fn ending_one_microphone_session_keeps_other_session_alive_and_rejects_late_packets() {
    let mut tracks = PlaybackTracks::default();
    let mut first = frame(MediaAudioSource::Microphone);
    let mut second = frame(MediaAudioSource::Microphone);
    first.session_track = true;
    second.session_track = true;
    let first_key = tracks.admit(&first).unwrap().key;
    let second_key = tracks.admit(&second).unwrap().key;

    assert_eq!(
        tracks.stop_microphone_session("sender", first.track_id),
        Some(first_key)
    );
    assert!(tracks.admit(&first).is_none());
    assert_eq!(tracks.admit(&second).unwrap().key, second_key);
}

#[test]
fn simultaneous_system_tracks_from_same_account_do_not_replace_each_other() {
    let mut tracks = PlaybackTracks::default();
    let microphone = frame(MediaAudioSource::Microphone);
    let first_system = frame(MediaAudioSource::System);
    let second_system = frame(MediaAudioSource::System);
    let microphone_key = tracks.admit(&microphone).unwrap().key;
    let first_key = tracks.admit(&first_system).unwrap().key;

    let second = tracks.admit(&second_system).unwrap();

    assert_ne!(first_key, second.key);
    assert!(second.replaced.is_none());
    assert_eq!(tracks.admit(&first_system).unwrap().key, first_key);
    assert_eq!(tracks.admit(&second_system).unwrap().key, second.key);
    assert_eq!(tracks.admit(&microphone).unwrap().key, microphone_key);
}

#[test]
fn stopping_one_system_track_keeps_other_device_audio_alive() {
    let mut tracks = PlaybackTracks::default();
    let first = frame(MediaAudioSource::System);
    let second = frame(MediaAudioSource::System);
    let first_key = tracks.admit(&first).unwrap().key;
    let second_key = tracks.admit(&second).unwrap().key;

    assert_eq!(
        tracks.stop_source(
            "sender",
            MediaAudioSource::System,
            first.voice_session_id,
            first.track_id
        ),
        Some(first_key)
    );
    assert!(tracks.admit(&first).is_none());
    assert_eq!(tracks.admit(&second).unwrap().key, second_key);
}

#[test]
fn stopping_system_track_rejects_queued_packets_and_allows_new_capture() {
    let mut tracks = PlaybackTracks::default();
    let system = frame(MediaAudioSource::System);
    let key = tracks.admit(&system).unwrap().key;

    let stopped = tracks.stop_source(
        "sender",
        MediaAudioSource::System,
        system.voice_session_id,
        system.track_id,
    );

    assert_eq!(stopped, Some(key));
    assert!(tracks.admit(&system).is_none());
    assert!(tracks.admit(&frame(MediaAudioSource::System)).is_some());
}

#[test]
fn old_stop_event_does_not_stop_restarted_system_track() {
    let mut tracks = PlaybackTracks::default();
    let previous = frame(MediaAudioSource::System);
    let current = frame(MediaAudioSource::System);
    let previous_key = tracks.admit(&previous).unwrap().key;
    let current_key = tracks.admit(&current).unwrap().key;

    assert_eq!(
        tracks.stop_source(
            "sender",
            MediaAudioSource::System,
            previous.voice_session_id,
            previous.track_id
        ),
        Some(previous_key)
    );

    assert_eq!(tracks.admit(&current).unwrap().key, current_key);
    assert!(tracks.admit(&previous).is_none());
}

#[test]
fn stop_before_first_packet_rejects_delayed_audio_from_ended_capture() {
    let mut tracks = PlaybackTracks::default();
    let ended = frame(MediaAudioSource::System);

    assert!(
        tracks
            .stop_source(
                "sender",
                MediaAudioSource::System,
                ended.voice_session_id,
                ended.track_id
            )
            .is_none()
    );

    assert!(tracks.admit(&ended).is_none());
    assert!(tracks.admit(&frame(MediaAudioSource::System)).is_some());
}

#[test]
fn changing_channels_with_same_track_id_is_rejected() {
    let mut tracks = PlaybackTracks::default();
    let mut system = frame(MediaAudioSource::System);
    tracks.admit(&system).unwrap();

    system.audio_channels = 1;

    assert!(tracks.admit(&system).is_none());
}

#[test]
fn sender_cleanup_does_not_remove_another_users_track() {
    let mut tracks = PlaybackTracks::default();
    let first = frame(MediaAudioSource::Microphone);
    let mut second = first.clone();
    second.sender_user_id = "second".into();
    tracks.admit(&first).unwrap();
    let second_key = tracks.admit(&second).unwrap().key;

    tracks.remove_sender("sender");

    assert!(
        !tracks
            .slots
            .contains_key(&("sender".into(), MediaAudioSource::Microphone, None))
    );
    assert_eq!(tracks.admit(&second).unwrap().key, second_key);
    assert_eq!(playback_owner(&second_key), "second");
}

#[test]
fn legacy_identity_is_allowed_only_for_microphone() {
    let mut tracks = PlaybackTracks::default();
    let mut microphone = frame(MediaAudioSource::Microphone);
    microphone.track_id = Uuid::nil();
    let mut system = frame(MediaAudioSource::System);
    system.track_id = Uuid::nil();

    assert!(tracks.admit(&microphone).is_some());
    assert!(tracks.admit(&system).is_none());
}

#[test]
fn rejoining_same_session_reactivates_microphone_without_stopping_other_device() {
    let mut tracks = PlaybackTracks::default();
    let mut first = frame(MediaAudioSource::Microphone);
    let mut second = frame(MediaAudioSource::Microphone);
    first.session_track = true;
    second.session_track = true;
    let first_key = tracks.admit(&first).unwrap().key;
    let second_key = tracks.admit(&second).unwrap().key;

    assert_eq!(
        tracks.stop_microphone_session("sender", first.track_id),
        Some(first_key.clone()),
    );
    assert!(tracks.admit(&first).is_none());
    assert_eq!(tracks.admit(&second).unwrap().key, second_key);

    tracks.reactivate_microphone_session("sender", first.track_id);

    assert_eq!(tracks.admit(&first).unwrap().key, first_key);
    assert_eq!(tracks.admit(&second).unwrap().key, second_key);
}

#[test]
fn identical_capture_ids_on_different_voice_sessions_stay_independent() {
    let mut tracks = PlaybackTracks::default();
    let first = frame(MediaAudioSource::System);
    let mut second = frame(MediaAudioSource::System);
    second.track_id = first.track_id;
    assert_ne!(first.voice_session_id, second.voice_session_id);

    let first_key = tracks.admit(&first).unwrap().key;
    let second_key = tracks.admit(&second).unwrap().key;
    assert_ne!(first_key, second_key);
    assert_eq!(tracks.admit(&first).unwrap().key, first_key);
    assert_eq!(tracks.admit(&second).unwrap().key, second_key);

    assert_eq!(
        tracks.stop_source(
            "sender",
            MediaAudioSource::System,
            first.voice_session_id,
            first.track_id,
        ),
        Some(first_key)
    );
    assert!(tracks.admit(&first).is_none());
    assert_eq!(tracks.admit(&second).unwrap().key, second_key);
}

fn frame(source: MediaAudioSource) -> VoiceFrame {
    VoiceFrame {
        sender_user_id: "sender".into(),
        track_id: Uuid::new_v4(),
        voice_session_id: (source == MediaAudioSource::System).then(Uuid::new_v4),
        session_track: false,
        audio_source: source,
        audio_channels: if source == MediaAudioSource::System {
            2
        } else {
            1
        },
        sequence: 0,
        timestamp_us: 0,
        duration_us: 20_000,
        codec: PlaybackCodec::Opus,
        bytes: vec![1],
    }
}
