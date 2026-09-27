use cheenhub_contracts::realtime::{LoadRoomHistory, SendMessage};
use cheenhub_contracts::rest::ServerRoomKind;
use uuid::Uuid;

use super::super::{TextChatApplicationError, load_room_history, send_message};
use super::{create_limited_server_room, create_server_room, registered_user, state};
use crate::features::servers::domain::ServerRole;
use crate::state::AppState;
/// Добавляет пользователя на сервер как активного участника.
async fn join_server(state: &AppState, server_id: &Uuid, user_id: &Uuid) {
    state
        .server_store
        .insert_server_member(server_id, user_id)
        .await
        .expect("member should insert");
}

/// Назначает пользователю роль на сервере.
async fn assign_role(state: &AppState, server_id: &Uuid, user_id: &Uuid, role_id: &Uuid) {
    state
        .server_store
        .assign_server_member_role(server_id, user_id, role_id, user_id)
        .await
        .expect("role should be assigned");
}

/// Создает сервер с одной комнатой, доступной для записи только владельцу.
async fn owner_only_room(state: &AppState, owner_user_id: &Uuid) -> (String, String) {
    create_limited_server_room(
        state,
        owner_user_id,
        "Read Only Server",
        "read-only",
        ServerRoomKind::TextAndVoice,
        Vec::new(),
    )
    .await
}

#[tokio::test]
async fn owner_can_send_and_load_room_messages() {
    let state = state();
    let auth = registered_user(&state, "chat_owner", "chat-owner@example.com").await;
    let user_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let (server_id, room_id) = create_server_room(
        &state,
        &user_id,
        "Chat Server",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;

    let accepted = send_message(
        &state,
        &auth.user,
        &user_id,
        SendMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            body: "  hello wt  ".to_owned(),
            attachment_ids: Vec::new(),
        },
    )
    .await
    .expect("send should be accepted");

    assert_eq!(accepted.message.body, "hello wt");
    tokio::task::yield_now().await;
    let history = load_room_history(
        &state,
        &user_id,
        LoadRoomHistory {
            server_id,
            room_id,
            before_message_id: None,
        },
    )
    .await
    .expect("history should load");

    assert_eq!(history.messages.len(), 1);
    assert_eq!(history.messages[0].id, accepted.message.id);
}

#[tokio::test]
async fn non_member_cannot_load_or_send() {
    let state = state();
    let owner = registered_user(&state, "chat_owner2", "chat-owner2@example.com").await;
    let outsider = registered_user(&state, "outsider", "outsider@example.com").await;
    let owner_id = Uuid::parse_str(&owner.user.id).expect("user id should be uuid");
    let (server_id, room_id) = create_server_room(
        &state,
        &owner_id,
        "Private Server",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let outsider_id = Uuid::parse_str(&outsider.user.id).expect("user id should be uuid");

    let load_error = load_room_history(
        &state,
        &outsider_id,
        LoadRoomHistory {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            before_message_id: None,
        },
    )
    .await
    .expect_err("non-member history should fail");
    let send_error = send_message(
        &state,
        &outsider.user,
        &outsider_id,
        SendMessage {
            server_id,
            room_id,
            body: "hello".to_owned(),
            attachment_ids: Vec::new(),
        },
    )
    .await
    .expect_err("non-member send should fail");

    assert!(matches!(
        load_error,
        TextChatApplicationError::Unauthorized(_)
    ));
    assert!(matches!(
        send_error,
        TextChatApplicationError::Unauthorized(_)
    ));
}

#[tokio::test]
async fn voice_room_rejects_text_chat() {
    let state = state();
    let auth = registered_user(&state, "voice_owner", "voice-owner@example.com").await;
    let user_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let (server_id, room_id) = create_server_room(
        &state,
        &user_id,
        "Voice Server",
        "Voice",
        ServerRoomKind::Voice,
    )
    .await;

    let error = send_message(
        &state,
        &auth.user,
        &user_id,
        SendMessage {
            server_id,
            room_id,
            body: "hello".to_owned(),
            attachment_ids: Vec::new(),
        },
    )
    .await
    .expect_err("voice room should reject text chat");

    assert!(matches!(error, TextChatApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn message_body_is_required_and_limited() {
    let state = state();
    let auth = registered_user(&state, "limit_owner", "limit-owner@example.com").await;
    let user_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let (server_id, room_id) = create_server_room(
        &state,
        &user_id,
        "Limits",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;

    for body in ["   ".to_owned(), "x".repeat(2001)] {
        let error = send_message(
            &state,
            &auth.user,
            &user_id,
            SendMessage {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
                body,
                attachment_ids: Vec::new(),
            },
        )
        .await
        .expect_err("invalid body should fail");

        assert!(matches!(error, TextChatApplicationError::BadRequest(_)));
    }
}

#[tokio::test]
async fn owner_can_write_in_room_without_selected_roles() {
    let state = state();
    let auth = registered_user(&state, "limited_owner", "limited-owner@example.com").await;
    let user_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let (server_id, room_id) = owner_only_room(&state, &user_id).await;

    let accepted = send_message(
        &state,
        &auth.user,
        &user_id,
        SendMessage {
            server_id,
            room_id,
            body: "owner message".to_owned(),
            attachment_ids: Vec::new(),
        },
    )
    .await
    .expect("owner should be able to write");

    assert_eq!(accepted.message.body, "owner message");
}

#[tokio::test]
async fn member_without_role_cannot_write_but_can_read() {
    let state = state();
    let owner = registered_user(
        &state,
        "limited_gate_owner",
        "limited-gate-owner@example.com",
    )
    .await;
    let member = registered_user(
        &state,
        "limited_gate_member",
        "limited-gate-member@example.com",
    )
    .await;
    let owner_id = Uuid::parse_str(&owner.user.id).expect("user id should be uuid");
    let member_id = Uuid::parse_str(&member.user.id).expect("user id should be uuid");
    let (server_id, room_id) = owner_only_room(&state, &owner_id).await;
    let server_uuid = Uuid::parse_str(&server_id).expect("server id should be uuid");
    join_server(&state, &server_uuid, &member_id).await;

    let error = send_message(
        &state,
        &member.user,
        &member_id,
        SendMessage {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
            body: "blocked".to_owned(),
            attachment_ids: Vec::new(),
        },
    )
    .await
    .expect_err("member without role should not write");

    assert!(matches!(error, TextChatApplicationError::ReadOnlyRoom(_)));

    let history = load_room_history(
        &state,
        &member_id,
        LoadRoomHistory {
            server_id,
            room_id,
            before_message_id: None,
        },
    )
    .await
    .expect("member should still read history");

    assert!(history.messages.is_empty());
}

#[tokio::test]
async fn member_with_selected_role_can_write() {
    let state = state();
    let owner = registered_user(
        &state,
        "limited_role_owner",
        "limited-role-owner@example.com",
    )
    .await;
    let member = registered_user(
        &state,
        "limited_role_member",
        "limited-role-member@example.com",
    )
    .await;
    let owner_id = Uuid::parse_str(&owner.user.id).expect("user id should be uuid");
    let member_id = Uuid::parse_str(&member.user.id).expect("user id should be uuid");
    let (server_id, room_id) = create_limited_server_room(
        &state,
        &owner_id,
        "Role Write Server",
        "editors",
        ServerRoomKind::TextAndVoice,
        Vec::new(),
    )
    .await;
    let server_uuid = Uuid::parse_str(&server_id).expect("server id should be uuid");
    let room_uuid = Uuid::parse_str(&room_id).expect("room id should be uuid");
    let role_id = Uuid::new_v4();
    state
        .server_store
        .replace_server_roles(
            &server_uuid,
            vec![ServerRole {
                id: role_id,
                server_id: server_uuid,
                name: "Редакторы".to_owned(),
                color: "#a855f7".to_owned(),
                kind: cheenhub_contracts::realtime::ServerRoleKind::Custom,
                position: 0,
                permissions: Vec::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }],
        )
        .await
        .expect("roles should be stored");
    state
        .server_store
        .update_server_room(
            &server_uuid,
            &room_uuid,
            "editors".to_owned(),
            ServerRoomKind::TextAndVoice,
            crate::features::servers::domain::ServerRoomWriteAccess {
                mode: cheenhub_contracts::rest::ServerRoomWriteAccessMode::SelectedRoles,
                role_ids: vec![role_id],
            },
        )
        .await
        .expect("room should be restricted");
    join_server(&state, &server_uuid, &member_id).await;
    assign_role(&state, &server_uuid, &member_id, &role_id).await;

    let accepted = send_message(
        &state,
        &member.user,
        &member_id,
        SendMessage {
            server_id,
            room_id,
            body: "editor message".to_owned(),
            attachment_ids: Vec::new(),
        },
    )
    .await
    .expect("member with role should be able to write");

    assert_eq!(accepted.message.body, "editor message");
}
