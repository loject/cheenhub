//! Регрессионные тесты realtime-контрактов приглашений в голосовую комнату.

use super::{
    ListVoiceRoomInviteCandidates, RespondVoiceRoomInvite, SendVoiceRoomInvite, VoiceChatKind,
    VoiceRoomInviteCandidate, VoiceRoomInviteLifecycleEvent, VoiceRoomInviteResponse,
    VoiceRoomInviteSnapshot, VoiceRoomInviteStatus,
};

#[test]
fn voice_room_invite_serialization_roundtrip() {
    let request = SendVoiceRoomInvite {
        server_id: "server-1".to_owned(),
        room_id: "room-2".to_owned(),
        recipient_user_id: "callee-3".to_owned(),
    };
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        serde_json::json!({
            "server_id": "server-1",
            "room_id": "room-2",
            "recipient_user_id": "callee-3"
        })
    );
    assert_eq!(
        serde_json::to_value(VoiceChatKind::SendVoiceRoomInvite).unwrap(),
        "send_voice_room_invite"
    );
    let response = RespondVoiceRoomInvite {
        invite_id: "invite-4".to_owned(),
        response: VoiceRoomInviteResponse::Accept,
    };
    assert_eq!(
        serde_json::to_value(&response).unwrap(),
        serde_json::json!({ "invite_id": "invite-4", "response": "accept" })
    );

    let invite = VoiceRoomInviteSnapshot {
        invite_id: "invite-4".to_owned(),
        server_id: "server-1".to_owned(),
        server_name: "Сервер".to_owned(),
        room_id: "room-2".to_owned(),
        room_name: "Голосовая".to_owned(),
        caller_user_id: "caller-5".to_owned(),
        caller_nickname: "Саша".to_owned(),
        caller_avatar_url: None,
        recipient_user_id: "callee-3".to_owned(),
        status: VoiceRoomInviteStatus::Pending,
        created_at: "2026-10-31T10:00:00Z".to_owned(),
        expires_at: "2026-10-31T10:00:45Z".to_owned(),
    };
    let event = VoiceRoomInviteLifecycleEvent {
        invite: invite.clone(),
    };
    let encoded = serde_json::to_value(&event).unwrap();
    assert_eq!(encoded["invite"]["status"], "pending");
    assert_eq!(encoded["invite"]["expires_at"], "2026-10-31T10:00:45Z");
    assert_eq!(
        serde_json::from_value::<VoiceRoomInviteLifecycleEvent>(encoded).unwrap(),
        event
    );
}

#[test]
fn voice_room_invite_candidates_expose_only_public_identity() {
    let request = ListVoiceRoomInviteCandidates {
        server_id: "server-1".to_owned(),
        room_id: "room-2".to_owned(),
    };
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        serde_json::json!({ "server_id": "server-1", "room_id": "room-2" })
    );
    let candidate = VoiceRoomInviteCandidate {
        user_id: "user-1".to_owned(),
        nickname: "Аня".to_owned(),
        avatar_url: None,
    };
    assert_eq!(
        serde_json::to_value(candidate).unwrap(),
        serde_json::json!({ "user_id": "user-1", "nickname": "Аня", "avatar_url": null })
    );
}
