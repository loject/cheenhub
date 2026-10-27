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
fn replacing_system_track_rejects_old_packets_and_preserves_microphone() {
    let mut tracks = PlaybackTracks::default();
    let microphone = frame(MediaAudioSource::Microphone);
    let old_system = frame(MediaAudioSource::System);
    let mic_key = tracks.admit(&microphone).unwrap().key;
    let old_key = tracks.admit(&old_system).unwrap().key;
    let new_system = frame(MediaAudioSource::System);

    let next = tracks.admit(&new_system).unwrap();

    assert_eq!(next.replaced, Some(old_key));
    assert!(tracks.admit(&old_system).is_none());
    assert_eq!(tracks.admit(&microphone).unwrap().key, mic_key);
}

#[test]
fn stopping_system_track_rejects_queued_packets_and_allows_new_capture() {
    let mut tracks = PlaybackTracks::default();
    let system = frame(MediaAudioSource::System);
    let key = tracks.admit(&system).unwrap().key;

    let stopped = tracks.stop_source("sender", MediaAudioSource::System, system.track_id);

    assert_eq!(stopped, Some(key));
    assert!(tracks.admit(&system).is_none());
    assert!(tracks.admit(&frame(MediaAudioSource::System)).is_some());
}

#[test]
fn old_stop_event_does_not_stop_restarted_system_track() {
    let mut tracks = PlaybackTracks::default();
    let previous = frame(MediaAudioSource::System);
    let current = frame(MediaAudioSource::System);
    tracks.admit(&previous).unwrap();
    let current_key = tracks.admit(&current).unwrap().key;

    assert!(
        tracks
            .stop_source("sender", MediaAudioSource::System, previous.track_id)
            .is_none()
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
            .stop_source("sender", MediaAudioSource::System, ended.track_id)
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
            .contains_key(&("sender".into(), MediaAudioSource::Microphone))
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

fn frame(source: MediaAudioSource) -> VoiceFrame {
    VoiceFrame {
        sender_user_id: "sender".into(),
        track_id: Uuid::new_v4(),
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
