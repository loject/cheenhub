//! Проверка удаления сервера его владельцем.

use cheenhub_contracts::rest::{AuthResponse, ServerSummary};
use uuid::Uuid;

use super::*;

/// Регистрирует пользователя с уникальным для теста почтовым адресом.
async fn register(state: &AppState, nickname: &str) -> AuthResponse {
    auth_application::register(
        state,
        RegisterRequest {
            nickname: nickname.to_owned(),
            email: format!("{nickname}@example.com"),
            password: "password123".to_owned(),
            accepts_terms: true,
            accepts_personal_data: true,
        },
    )
    .await
    .expect("registration should succeed")
}

/// Создает сервер для указанного пользователя.
async fn create_server(state: &AppState, access_token: &str, name: &str) -> ServerSummary {
    create(
        state,
        access_token,
        CreateServerRequest {
            name: name.to_owned(),
        },
    )
    .await
    .expect("server creation should succeed")
    .server
}

#[tokio::test]
async fn owner_can_delete_own_server() {
    let state = state();
    let owner = register(&state, "delete_owner").await;
    let kept = create_server(&state, &owner.access_token, "Kept Hub").await;
    let deleted = create_server(&state, &owner.access_token, "Deleted Hub").await;

    delete(&state, &owner.access_token, deleted.id.clone())
        .await
        .expect("owner should be able to delete own server");

    let listed = list(&state, &owner.access_token)
        .await
        .expect("server list should load");
    assert_eq!(
        listed
            .servers
            .iter()
            .map(|server| server.id.as_str())
            .collect::<Vec<_>>(),
        vec![kept.id.as_str()]
    );

    let error = delete(&state, &owner.access_token, deleted.id.clone())
        .await
        .expect_err("repeated deletion should report a missing server");
    assert!(matches!(error, ServerError::NotFound(_)));
}

#[tokio::test]
async fn deleted_server_loses_rooms_and_invites() {
    let state = state();
    let owner = register(&state, "delete_cascade_owner").await;
    let server = create_server(&state, &owner.access_token, "Cascade Hub").await;
    create_invite(
        &state,
        &owner.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite creation should succeed");
    let server_id = Uuid::parse_str(&server.id).expect("server id should be uuid");

    delete(&state, &owner.access_token, server.id.clone())
        .await
        .expect("server deletion should succeed");

    let rooms = state
        .server_store
        .list_server_rooms(&server_id)
        .await
        .expect("rooms should be readable");
    let invites = state
        .server_store
        .list_server_invites(&server_id)
        .await
        .expect("invites should be readable");
    let roles = state
        .server_store
        .list_server_roles(&server_id)
        .await
        .expect("roles should be readable");
    assert!(rooms.is_empty());
    assert!(invites.is_empty());
    assert!(roles.is_empty());
}

#[tokio::test]
async fn member_cannot_delete_server_of_another_owner() {
    let state = state();
    let owner = register(&state, "delete_foreign_owner").await;
    let member = register(&state, "delete_foreign_member").await;
    let server = create_server(&state, &owner.access_token, "Foreign Hub").await;
    let invite = create_invite(
        &state,
        &owner.access_token,
        server.id.clone(),
        CreateServerInviteRequest {
            max_uses: None,
            expires_in_days: None,
        },
    )
    .await
    .expect("invite creation should succeed");
    accept_invite(&state, &member.access_token, invite.code.clone())
        .await
        .expect("member should join through the invite");

    let error = delete(&state, &member.access_token, server.id.clone())
        .await
        .expect_err("member should not be able to delete the server");

    assert!(matches!(error, ServerError::NotFound(_)));
    let listed = list(&state, &owner.access_token)
        .await
        .expect("server list should load");
    assert_eq!(listed.servers.len(), 1);
    assert_eq!(listed.servers[0].id, server.id);
}

#[tokio::test]
async fn guest_cannot_delete_server_they_have_no_access_to() {
    let state = state();
    let owner = register(&state, "delete_hidden_owner").await;
    let guest = register(&state, "delete_hidden_guest").await;
    let server = create_server(&state, &owner.access_token, "Hidden Hub").await;

    let error = delete(&state, &guest.access_token, server.id.clone())
        .await
        .expect_err("guest should not be able to delete a foreign server");

    assert!(matches!(error, ServerError::NotFound(_)));
    let listed = list(&state, &owner.access_token)
        .await
        .expect("server list should load");
    assert_eq!(listed.servers.len(), 1);
}

#[tokio::test]
async fn server_deletion_rejects_malformed_server_id() {
    let state = state();
    let owner = register(&state, "delete_malformed_id").await;

    let error = delete(&state, &owner.access_token, "not-a-uuid".to_owned())
        .await
        .expect_err("malformed server id should be rejected");

    assert!(matches!(error, ServerError::BadRequest(_)));
}

#[tokio::test]
async fn deleting_server_removes_only_its_voice_presence() {
    use crate::features::voice_chat::infrastructure::{VoicePresence, VoicePresenceTargetKind};

    let state = state();
    let owner = register(&state, "delete_voice_owner").await;
    let server = create_server(&state, &owner.access_token, "Voice Hub").await;
    let server_id = Uuid::parse_str(&server.id).unwrap();
    let room_id = Uuid::new_v4();
    let user_id = Uuid::parse_str(&owner.user.id).unwrap();
    let session_id = Uuid::new_v4();
    let stream_id = Uuid::new_v4();
    let (outbound, mut notifications) = tokio::sync::mpsc::channel(2);
    state
        .realtime_hub
        .register_stream(
            stream_id,
            cheenhub_contracts::realtime::RealtimeModule::VoiceChat,
            user_id,
            crate::realtime::EnvelopeSink::websocket(outbound),
        )
        .await;
    for (kind, target_server_id, target_room_id, target_user_id) in [
        (VoicePresenceTargetKind::Server, server_id, room_id, user_id),
        (
            VoicePresenceTargetKind::Server,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        ),
        (
            VoicePresenceTargetKind::DirectMessage,
            server_id,
            room_id,
            Uuid::new_v4(),
        ),
    ] {
        state
            .voice_presence_store
            .join(VoicePresence {
                realtime_stream_id: if target_user_id == user_id {
                    stream_id
                } else {
                    Uuid::new_v4()
                },
                session_id,
                target_kind: kind,
                server_id: target_server_id,
                room_id: target_room_id,
                user_id: target_user_id,
                nickname: "Участник".to_owned(),
                avatar_url: None,
                joined_at: chrono::Utc::now(),
            })
            .await;
    }

    delete(&state, &owner.access_token, server.id.clone())
        .await
        .unwrap();

    assert!(
        state
            .voice_presence_store
            .room_presence_for_user(VoicePresenceTargetKind::Server, &room_id, &user_id,)
            .is_none()
    );
    assert_eq!(
        state
            .voice_presence_store
            .active_voice_connection_count()
            .await,
        2
    );
    let notification = notifications
        .try_recv()
        .expect("участник получает завершение присутствия");
    let crate::realtime::WebSocketOutbound::Envelope(envelope) = notification else {
        panic!("ожидался снимок участников");
    };
    let snapshot: cheenhub_contracts::realtime::VoiceRoomSnapshot =
        serde_json::from_value(envelope.payload).unwrap();
    assert_eq!(snapshot.room_id, room_id.to_string());
    assert!(snapshot.participants.is_empty());
}
