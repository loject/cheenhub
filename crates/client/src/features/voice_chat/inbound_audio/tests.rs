//! Регрессии фильтрации входящего звука нескольких устройств одного аккаунта.

use super::*;
use cheenhub_contracts::media::MediaAudioSource;

#[test]
fn other_device_microphone_is_playable_but_own_microphone_is_not() {
    let own_session = Uuid::from_u128(1);
    let other_session = Uuid::from_u128(2);

    assert!(!is_own_microphone_frame(
        "same-user",
        "same-user",
        MediaAudioSource::Microphone,
        other_session,
        Some(own_session),
    ));
    assert!(is_own_microphone_frame(
        "same-user",
        "same-user",
        MediaAudioSource::Microphone,
        own_session,
        Some(own_session),
    ));
    assert!(!is_own_microphone_frame(
        "same-user",
        "same-user",
        MediaAudioSource::System,
        other_session,
        Some(own_session),
    ));
    assert!(!is_own_microphone_frame(
        "different-user",
        "same-user",
        MediaAudioSource::Microphone,
        own_session,
        Some(own_session),
    ));
}

#[test]
fn leaving_one_voice_session_stops_only_its_microphone() {
    let first = participant("same-user", Uuid::from_u128(101), "joined-first");
    let second = participant("same-user", Uuid::from_u128(102), "joined-second");

    let AudioSessionChanges { stopped, restarted } =
        audio_session_changes(&[first, second.clone()], &[second]);

    assert_eq!(
        stopped,
        vec![("same-user".to_owned(), Uuid::from_u128(101))]
    );
    assert!(restarted.is_empty());
}

#[test]
fn reappearing_voice_session_restarts_the_same_microphone_track() {
    let returned = participant("same-user", Uuid::from_u128(101), "joined-again");

    let AudioSessionChanges { stopped, restarted } = audio_session_changes(&[], &[returned]);

    assert!(stopped.is_empty());
    assert_eq!(
        restarted,
        vec![("same-user".to_owned(), Uuid::from_u128(101))]
    );
}

#[test]
fn changed_join_time_restarts_same_session_even_without_missing_snapshot() {
    let old = participant("same-user", Uuid::from_u128(101), "joined-first");
    let new = participant("same-user", Uuid::from_u128(101), "joined-again");

    let AudioSessionChanges { stopped, restarted } = audio_session_changes(&[old], &[new]);

    assert_eq!(
        stopped,
        vec![("same-user".to_owned(), Uuid::from_u128(101))]
    );
    assert_eq!(
        restarted,
        vec![("same-user".to_owned(), Uuid::from_u128(101))]
    );
}

#[test]
fn unchanged_presence_and_legacy_missing_session_do_not_restart_audio() {
    let current = participant("same-user", Uuid::from_u128(101), "joined-first");
    let mut legacy = current.clone();
    legacy.session_id.clear();

    let AudioSessionChanges { stopped, restarted } =
        audio_session_changes(&[current.clone(), legacy.clone()], &[current, legacy]);

    assert!(stopped.is_empty());
    assert!(restarted.is_empty());
}

fn participant(user_id: &str, session_id: Uuid, joined_at: &str) -> VoiceRoomParticipant {
    VoiceRoomParticipant {
        microphone_enabled: true,
        output_muted: false,
        user_id: user_id.to_owned(),
        session_id: session_id.to_string(),
        voice_protocol_version: 4,
        device_kind: cheenhub_contracts::realtime::VoiceDeviceKind::Desktop,
        nickname: "Участник".to_owned(),
        avatar_url: None,
        joined_at: joined_at.to_owned(),
    }
}

#[test]
fn legacy_devices_of_same_user_are_independent_microphone_sessions() {
    use cheenhub_contracts::realtime::VoiceDeviceKind;

    let first_session = Uuid::from_u128(101);
    let second_session = Uuid::from_u128(102);
    let participants = [first_session, second_session]
        .into_iter()
        .map(|session_id| VoiceRoomParticipant {
            microphone_enabled: true,
            output_muted: false,
            user_id: "same-user".to_owned(),
            session_id: session_id.to_string(),
            voice_protocol_version: 4,
            device_kind: VoiceDeviceKind::Desktop,
            nickname: "Участник".to_owned(),
            avatar_url: None,
            joined_at: String::new(),
        })
        .collect::<Vec<_>>();

    assert!(is_session_microphone_frame(
        &participants,
        "same-user",
        first_session,
    ));
    assert!(is_session_microphone_frame(
        &participants,
        "same-user",
        second_session,
    ));
    assert!(!is_session_microphone_frame(
        &participants,
        "another-user",
        first_session,
    ));
    assert!(!is_session_microphone_frame(
        &participants,
        "same-user",
        Uuid::from_u128(103),
    ));
}
