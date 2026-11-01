//! Регрессионные тесты realtime-контрактов приглашений в голосовую комнату.

use super::super::VoiceChatKind;
use super::{
    ListVoiceRoomInviteCandidates, RespondVoiceRoomInvite, SendVoiceRoomInvite,
    VoiceRoomInviteCandidate, VoiceRoomInviteLifecycleEvent, VoiceRoomInviteResponse,
    VoiceRoomInviteSnapshot, VoiceRoomInviteStatus, VoiceRoomInvitesSnapshot,
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
        recipient_nickname: None,
        recipient_avatar_url: None,
        status: VoiceRoomInviteStatus::Pending,
        created_at: "2026-10-31T10:00:00Z".to_owned(),
        expires_at: "2026-10-31T10:00:45Z".to_owned(),
        resolved_at: None,
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
fn voice_room_invite_snapshot_roundtrips_recipient_metadata() {
    let encoded = serde_json::json!({
        "invite_id": "invite-4",
        "server_id": "server-1",
        "server_name": "Сервер",
        "room_id": "room-2",
        "room_name": "Голосовая",
        "caller_user_id": "caller-5",
        "caller_nickname": "Саша",
        "caller_avatar_url": null,
        "recipient_user_id": "callee-3",
        "recipient_nickname": "Аня",
        "recipient_avatar_url": "/avatars/anna.png",
        "status": "declined",
        "created_at": "2026-10-31T10:00:00Z",
        "expires_at": "2026-10-31T10:00:45Z",
        "resolved_at": "2026-10-31T10:00:12Z"
    });

    let decoded: VoiceRoomInviteSnapshot = serde_json::from_value(encoded.clone()).unwrap();

    assert_eq!(serde_json::to_value(decoded).unwrap(), encoded);
}

#[test]
fn legacy_voice_room_invite_snapshot_omits_new_optional_metadata() {
    let legacy = serde_json::json!({
        "invite_id": "invite-4",
        "server_id": "server-1",
        "server_name": "Сервер",
        "room_id": "room-2",
        "room_name": "Голосовая",
        "caller_user_id": "caller-5",
        "caller_nickname": "Саша",
        "caller_avatar_url": null,
        "recipient_user_id": "callee-3",
        "status": "pending",
        "created_at": "2026-10-31T10:00:00Z",
        "expires_at": "2026-10-31T10:00:45Z"
    });

    let decoded: VoiceRoomInviteSnapshot = serde_json::from_value(legacy.clone()).unwrap();

    assert!(decoded.recipient_nickname.is_none());
    assert!(decoded.recipient_avatar_url.is_none());
    assert!(decoded.resolved_at.is_none());
    assert_eq!(serde_json::to_value(decoded).unwrap(), legacy);
}

#[test]
fn recovered_invites_include_server_time_and_accept_legacy_snapshot() {
    let legacy = serde_json::json!({ "invites": [] });
    let old: VoiceRoomInvitesSnapshot = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(old.server_now, None);
    assert_eq!(serde_json::to_value(old).unwrap(), legacy);

    let current = serde_json::json!({
        "invites": [],
        "server_now": "2026-10-31T15:00:40Z"
    });
    let restored: VoiceRoomInvitesSnapshot = serde_json::from_value(current.clone()).unwrap();
    assert_eq!(restored.server_now.as_deref(), Some("2026-10-31T15:00:40Z"));
    assert_eq!(serde_json::to_value(restored).unwrap(), current);
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
