//! Регрессии распределения видеодорожек по голосовым сессиям одного пользователя.

use super::*;

fn participant(user_id: &str, session_id: Uuid) -> VoiceRoomParticipant {
    VoiceRoomParticipant {
        microphone_enabled: true,
        output_muted: false,
        user_id: user_id.to_owned(),
        session_id: session_id.to_string(),
        voice_protocol_version: 5,
        device_kind: cheenhub_contracts::realtime::VoiceDeviceKind::Desktop,
        nickname: user_id.to_owned(),
        avatar_url: None,
        joined_at: "2026-10-30T00:00:00Z".to_owned(),
    }
}

#[test]
fn speech_indicator_applies_only_to_matching_voice_session() {
    let first_id = Uuid::from_u128(1);
    let second_id = Uuid::from_u128(2);
    let tiles = participant_tiles(
        &[
            participant("same", first_id),
            participant("same", second_id),
        ],
        &[("same".to_owned(), Some(first_id))],
        &[],
        &[],
        "someone_else",
        None,
        false,
    );
    assert!(tiles[0].speaking);
    assert!(!tiles[1].speaking);
}

#[test]
fn legacy_participants_without_session_ids_still_show_video_and_local_preview() {
    let mut current = participant("self", Uuid::nil());
    current.session_id.clear();
    let mut remote = participant("peer", Uuid::nil());
    remote.session_id.clear();
    let remote_track = Uuid::new_v4();
    let tiles = participant_tiles(
        &[current, remote],
        &[],
        &[("peer".to_owned(), remote_track)],
        &[],
        "self",
        Some(Uuid::new_v4()),
        true,
    );

    assert_eq!(tiles.len(), 2);
    assert_eq!(tiles[0].media, VoiceParticipantTileMedia::Camera);
    assert!(tiles[0].track_id.is_nil());
    assert_eq!(tiles[1].media, VoiceParticipantTileMedia::Camera);
    assert_eq!(tiles[1].track_id, remote_track);
}

#[test]
fn same_user_sessions_receive_only_their_own_video_tiles() {
    let first_id = Uuid::from_u128(1);
    let second_id = Uuid::from_u128(2);
    let participants = [
        participant("same", first_id),
        participant("same", second_id),
    ];
    let tiles = participant_tiles(
        &participants,
        &[],
        &[("same".to_owned(), first_id)],
        &[],
        "someone_else",
        None,
        false,
    );

    assert_eq!(tiles.len(), 2);
    assert_ne!(tiles[0].key, tiles[1].key);
    assert_eq!(tiles[0].media, VoiceParticipantTileMedia::Camera);
    assert_eq!(tiles[0].track_id, first_id);
    assert_eq!(tiles[1].media, VoiceParticipantTileMedia::Avatar);
    assert!(tiles[1].track_id.is_nil());
}

#[test]
fn local_preview_is_assigned_only_to_the_current_voice_session() {
    let other_id = Uuid::from_u128(1);
    let current_id = Uuid::from_u128(2);
    let participants = [
        participant("same", other_id),
        participant("same", current_id),
    ];
    let tiles = participant_tiles(
        &participants,
        &[],
        &[("same".to_owned(), Uuid::nil())],
        &[],
        "same",
        Some(current_id),
        true,
    );

    assert_eq!(tiles.len(), 2);
    assert_eq!(tiles[0].media, VoiceParticipantTileMedia::Avatar);
    assert_eq!(tiles[1].media, VoiceParticipantTileMedia::Camera);
    assert!(tiles[1].track_id.is_nil());
}
