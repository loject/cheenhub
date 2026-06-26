use cheenhub_contracts::realtime::LoadRoomHistory;
use cheenhub_contracts::rest::ServerRoomKind;
use chrono::{Duration, Utc};
use uuid::Uuid;

use super::super::{TextChatApplicationError, load_room_history};
use super::{create_server_room, registered_user, state};
use crate::features::text_chat::domain::TextMessage;

#[tokio::test]
async fn room_history_returns_latest_fifty_oldest_to_newest() {
    let state = state();
    let auth = registered_user(&state, "history_owner", "history-owner@example.com").await;
    let user_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let (server_id_string, room_id_string) = create_server_room(
        &state,
        &user_id,
        "History",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let server_id = Uuid::parse_str(&server_id_string).expect("server id should be uuid");
    let room_id = Uuid::parse_str(&room_id_string).expect("room id should be uuid");
    let base_time = Utc::now();

    for index in 0..55 {
        state
            .text_chat_store
            .insert_text_message(TextMessage {
                id: Uuid::new_v4(),
                server_id,
                room_id,
                author_user_id: user_id,
                author_nickname: auth.user.nickname.clone(),
                body: format!("message {index}"),
                attachments: Vec::new(),
                created_at: base_time + Duration::seconds(index),
                deleted_at: None,
                deleted_by_user_id: None,
            })
            .await
            .expect("message should insert");
    }

    let history = load_room_history(
        &state,
        &user_id,
        LoadRoomHistory {
            server_id: server_id_string,
            room_id: room_id_string,
            before_message_id: None,
        },
    )
    .await
    .expect("history should load");

    assert_eq!(history.messages.len(), 50);
    assert!(history.has_more);
    assert_eq!(history.messages[0].body, "message 5");
    assert_eq!(history.messages[49].body, "message 54");
}

#[tokio::test]
async fn room_history_page_before_cursor_returns_older_messages() {
    let state = state();
    let auth = registered_user(&state, "cursor_owner", "cursor-owner@example.com").await;
    let user_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let (server_id_string, room_id_string) = create_server_room(
        &state,
        &user_id,
        "Cursor History",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let server_id = Uuid::parse_str(&server_id_string).expect("server id should be uuid");
    let room_id = Uuid::parse_str(&room_id_string).expect("room id should be uuid");
    let base_time = Utc::now();

    for index in 0..75 {
        state
            .text_chat_store
            .insert_text_message(TextMessage {
                id: Uuid::new_v4(),
                server_id,
                room_id,
                author_user_id: user_id,
                author_nickname: auth.user.nickname.clone(),
                body: format!("message {index}"),
                attachments: Vec::new(),
                created_at: base_time + Duration::seconds(index),
                deleted_at: None,
                deleted_by_user_id: None,
            })
            .await
            .expect("message should insert");
    }

    let latest = load_room_history(
        &state,
        &user_id,
        LoadRoomHistory {
            server_id: server_id_string.clone(),
            room_id: room_id_string.clone(),
            before_message_id: None,
        },
    )
    .await
    .expect("latest history should load");
    let cursor = latest.messages[0].id.clone();
    let older = load_room_history(
        &state,
        &user_id,
        LoadRoomHistory {
            server_id: server_id_string,
            room_id: room_id_string,
            before_message_id: Some(cursor),
        },
    )
    .await
    .expect("older history should load");

    assert_eq!(latest.messages[0].body, "message 25");
    assert_eq!(older.messages.len(), 25);
    assert!(!older.has_more);
    assert_eq!(older.messages[0].body, "message 0");
    assert_eq!(older.messages[24].body, "message 24");
}

#[tokio::test]
async fn foreign_history_cursor_is_rejected() {
    let state = state();
    let auth = registered_user(&state, "foreign_owner", "foreign-owner@example.com").await;
    let user_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let (server_id_string, room_id_string) = create_server_room(
        &state,
        &user_id,
        "Foreign Cursor",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let (_, other_room_id_string) = create_server_room(
        &state,
        &user_id,
        "Other Cursor",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let server_id = Uuid::parse_str(&server_id_string).expect("server id should be uuid");
    let other_room_id = Uuid::parse_str(&other_room_id_string).expect("room id should be uuid");
    let foreign_message_id = Uuid::new_v4();

    state
        .text_chat_store
        .insert_text_message(TextMessage {
            id: foreign_message_id,
            server_id,
            room_id: other_room_id,
            author_user_id: user_id,
            author_nickname: auth.user.nickname.clone(),
            body: "foreign".to_owned(),
            attachments: Vec::new(),
            created_at: Utc::now(),
            deleted_at: None,
            deleted_by_user_id: None,
        })
        .await
        .expect("message should insert");

    let error = load_room_history(
        &state,
        &user_id,
        LoadRoomHistory {
            server_id: server_id_string,
            room_id: room_id_string,
            before_message_id: Some(foreign_message_id.to_string()),
        },
    )
    .await
    .expect_err("foreign cursor should fail");

    assert!(matches!(error, TextChatApplicationError::BadRequest(_)));
}
