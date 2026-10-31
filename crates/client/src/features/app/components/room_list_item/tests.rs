//! Проверки ключей голосовых участников в боковом списке комнат.

use super::*;
use cheenhub_contracts::realtime::VoiceDeviceKind;
use uuid::Uuid;

#[test]
fn two_sessions_of_one_account_have_distinct_sidebar_render_keys() {
    let first = VoiceRoomParticipant {
        user_id: Uuid::from_u128(1).to_string(),
        session_id: Uuid::from_u128(2).to_string(),
        microphone_enabled: true,
        output_muted: false,
        voice_protocol_version: 6,
        device_kind: VoiceDeviceKind::Desktop,
        nickname: "Один аккаунт".to_owned(),
        avatar_url: None,
        joined_at: "2026-10-31T00:00:00Z".to_owned(),
    };
    let second = VoiceRoomParticipant {
        session_id: Uuid::from_u128(3).to_string(),
        ..first.clone()
    };

    assert_eq!(first.user_id, second.user_id);
    assert_ne!(
        voice_participant_key(&first),
        voice_participant_key(&second)
    );
    assert_eq!(voice_participant_key(&first), voice_participant_key(&first));
}

#[test]
fn joining_voice_room_expands_collapsed_participants() {
    assert!(expanded_after_connection_change(false, false, true));
}

#[test]
fn participant_updates_preserve_manual_collapse() {
    assert!(!expanded_after_connection_change(false, true, true));
}

#[test]
fn leaving_voice_room_does_not_force_expansion() {
    assert!(!expanded_after_connection_change(false, true, false));
}

#[test]
fn manual_expansion_survives_presence_changes() {
    assert!(expanded_after_connection_change(true, true, false));
}
