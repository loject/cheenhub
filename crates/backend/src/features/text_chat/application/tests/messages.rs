use cheenhub_contracts::realtime::{LoadRoomHistory, SendMessage};
use cheenhub_contracts::rest::ServerRoomKind;
use uuid::Uuid;

use super::super::{TextChatApplicationError, load_room_history, send_message};
use super::{create_server_room, registered_user, state};

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
