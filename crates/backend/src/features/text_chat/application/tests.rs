use std::sync::Arc;

use cheenhub_contracts::realtime::{DeleteMessage, LoadRoomHistory, SendMessage};
use cheenhub_contracts::rest::{RegisterRequest, ServerRoomKind};
use chrono::{Duration, Utc};
use image::ImageEncoder;
use image::codecs::png::PngEncoder;
use uuid::Uuid;

use super::{chat_image, delete_message, load_room_history, send_message, upload_chat_image};
use crate::features::auth::application as auth_application;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::text_chat::application::TextChatApplicationError;
use crate::features::text_chat::domain::TextMessage;
use crate::features::text_chat::infrastructure::InMemoryTextChatStore;
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;

fn state() -> AppState {
    AppState {
        auth_store: Arc::new(InMemoryAuthStore::default()),
        auth_mailer: Arc::new(crate::features::auth::email::tests::TestAuthMailer::default()),
        server_store: Arc::new(InMemoryServerStore::default()),
        text_chat_store: Arc::new(InMemoryTextChatStore::default()),
        chat_attachment_object_store: Arc::new(
            crate::features::text_chat::infrastructure::InMemoryChatAttachmentObjectStore::new(
                "test-chat-images",
            ),
        ),
        image_store: Arc::new(
            crate::features::images::infrastructure::InMemoryImageStore::default(),
        ),
        image_processing_queue: Arc::new(tokio::sync::Semaphore::new(1)),
        voice_presence_store: Arc::new(
            crate::features::voice_chat::infrastructure::InMemoryVoicePresenceStore::default(),
        ),
        realtime_hub: Arc::new(RealtimeHub::default()),
        auth_keys: AuthKeys::generate_for_tests(),
        access_token_lifetime_minutes: 15,
        refresh_token_lifetime_days: 30,
        google_oauth_client_id: Some("test-google-client".to_owned()),
        google_oauth_client_secret: Some("test-google-secret".to_owned()),
        google_oauth_redirect_uri: Some(
            "http://localhost/api/auth/oauth/google/callback".to_owned(),
        ),
        cheenhub_client_base_url: "http://localhost".to_owned(),
        cheenhub_api_base_url: "http://localhost/api".to_owned(),
        oauth_state_lifetime_minutes: 10,
        oauth_handoff_lifetime_minutes: 5,
        oauth_registration_lifetime_minutes: 15,
        password_reset_token_lifetime_minutes: 30,
    }
}

async fn registered_user(
    state: &AppState,
    nickname: &str,
    email: &str,
) -> cheenhub_contracts::rest::AuthResponse {
    auth_application::register(
        state,
        RegisterRequest {
            nickname: nickname.to_owned(),
            email: email.to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed")
}

async fn create_server_room(
    state: &AppState,
    owner_user_id: &Uuid,
    server_name: &str,
    room_name: &str,
    room_kind: ServerRoomKind,
) -> (String, String) {
    let server = state
        .server_store
        .insert_server(owner_user_id, server_name.to_owned())
        .await
        .expect("server should insert");
    state
        .server_store
        .insert_server_member(&server.id, owner_user_id)
        .await
        .expect("member should insert");
    let room = state
        .server_store
        .insert_server_room(&server.id, room_name.to_owned(), room_kind)
        .await
        .expect("room should insert");

    (server.id.to_string(), room.id.to_string())
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

#[tokio::test]
async fn chat_image_upload_is_stored_and_served_through_proxy_flow() {
    let state = state();
    let auth = registered_user(&state, "image_owner", "image-owner@example.com").await;
    let user_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let (server_id, room_id) = create_server_room(
        &state,
        &user_id,
        "Images",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let bytes = tiny_png();

    let uploaded = upload_chat_image(
        &state,
        &user_id,
        server_id.clone(),
        room_id.clone(),
        Some("pixel.png".to_owned()),
        &bytes,
    )
    .await
    .expect("chat image should upload");

    assert_eq!(uploaded.server_id, server_id);
    assert_eq!(uploaded.room_id, room_id);
    assert_eq!(uploaded.content_type, "image/png");
    assert_eq!(uploaded.width, 1);
    assert_eq!(uploaded.height, 1);

    let (attachment, served) = chat_image(&state, &user_id, uploaded.id)
        .await
        .expect("chat image should serve");
    assert_eq!(attachment.content_type, "image/png");
    assert_eq!(served, bytes);
}

#[tokio::test]
async fn cannot_delete_message_from_foreign_server_via_owned_room() {
    let state = state();
    let attacker = registered_user(&state, "attacker", "attacker@example.com").await;
    let victim = registered_user(&state, "victim_del", "victim-del@example.com").await;
    let attacker_id = Uuid::parse_str(&attacker.user.id).expect("user id should be uuid");
    let victim_id = Uuid::parse_str(&victim.user.id).expect("user id should be uuid");

    // Атакующий владеет собственным сервером и текстовой комнатой.
    let (attacker_server, attacker_room) = create_server_room(
        &state,
        &attacker_id,
        "Attacker",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;

    // Жертва владеет отдельным сервером с сообщением, автор которого — жертва.
    let (victim_server_string, victim_room_string) = create_server_room(
        &state,
        &victim_id,
        "Victim",
        "general",
        ServerRoomKind::TextAndVoice,
    )
    .await;
    let victim_server = Uuid::parse_str(&victim_server_string).expect("server id should be uuid");
    let victim_room = Uuid::parse_str(&victim_room_string).expect("room id should be uuid");
    let foreign_message_id = Uuid::new_v4();
    state
        .text_chat_store
        .insert_text_message(TextMessage {
            id: foreign_message_id,
            server_id: victim_server,
            room_id: victim_room,
            author_user_id: victim_id,
            author_nickname: victim.user.nickname.clone(),
            body: "secret".to_owned(),
            attachments: Vec::new(),
            created_at: Utc::now(),
            deleted_at: None,
            deleted_by_user_id: None,
        })
        .await
        .expect("message should insert");

    // Атакующий — владелец СВОЕГО сервера, поэтому can_delete_any_message для него true.
    // Но удаление должно быть привязано к комнате сообщения, а не к запрошенной.
    let error = delete_message(
        &state,
        &attacker_id,
        DeleteMessage {
            server_id: attacker_server,
            room_id: attacker_room,
            message_id: foreign_message_id.to_string(),
        },
    )
    .await
    .expect_err("cross-server delete must be rejected");
    assert!(matches!(error, TextChatApplicationError::NotFound(_)));

    // Сообщение жертвы должно остаться нетронутым.
    let history = load_room_history(
        &state,
        &victim_id,
        LoadRoomHistory {
            server_id: victim_server_string,
            room_id: victim_room_string,
            before_message_id: None,
        },
    )
    .await
    .expect("victim history should load");
    assert_eq!(history.messages.len(), 1);
    assert_eq!(history.messages[0].id, foreign_message_id.to_string());
}

fn tiny_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(&[255, 255, 255, 255], 1, 1, image::ExtendedColorType::Rgba8)
        .expect("test png should encode");
    bytes
}
