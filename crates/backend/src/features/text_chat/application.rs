//! Text chat application flows.

use cheenhub_contracts::realtime::{
    LoadRoomHistory, RealtimeKind, RealtimeModule, RoomHistory, SendMessage, SendMessageAccepted,
    TextChatKind, TextChatMessage,
};
use cheenhub_contracts::rest::AuthUser;
use cheenhub_contracts::rest::ServerRoomKind;
use chrono::Utc;
use tokio::time::{Duration as TokioDuration, sleep};
use tracing::error;
use uuid::Uuid;

use crate::features::text_chat::domain::TextMessage;
use crate::features::text_chat::policy;
use crate::features::text_chat::validation;
use crate::state::AppState;

/// Loads the latest text messages for a room.
pub(crate) async fn load_room_history(
    state: &AppState,
    user_id: &Uuid,
    request: LoadRoomHistory,
) -> Result<RoomHistory, TextChatApplicationError> {
    let server_id = parse_id(&request.server_id, "Сервер не найден.")?;
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    let before_message_id = request
        .before_message_id
        .as_deref()
        .map(|value| parse_id(value, "История сообщений недоступна."))
        .transpose()?;
    ensure_room_text_available(state, user_id, &server_id, &room_id).await?;
    sleep(TokioDuration::from_millis(700)).await;
    let page = state
        .text_chat_store
        .room_message_page(&room_id, before_message_id.as_ref())
        .await
        .map_err(|error| {
            if before_message_id.is_some() {
                TextChatApplicationError::BadRequest("История сообщений недоступна.".to_owned())
            } else {
                TextChatApplicationError::Internal(error)
            }
        })?;

    Ok(RoomHistory {
        server_id: server_id.to_string(),
        room_id: room_id.to_string(),
        messages: page.messages.iter().map(message_summary).collect(),
        has_more: page.has_more,
    })
}

/// Accepts a message, starts fanout and persistence, and returns immediately.
pub(crate) async fn send_message(
    state: &AppState,
    user: &AuthUser,
    user_id: &Uuid,
    request: SendMessage,
) -> Result<SendMessageAccepted, TextChatApplicationError> {
    let server_id = parse_id(&request.server_id, "Сервер не найден.")?;
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    ensure_room_text_available(state, user_id, &server_id, &room_id).await?;
    let valid = validation::message_body(request.body)
        .map_err(|message| TextChatApplicationError::BadRequest(message.to_owned()))?;
    let message = TextMessage {
        id: Uuid::new_v4(),
        server_id,
        room_id,
        author_user_id: *user_id,
        author_nickname: user.nickname.clone(),
        body: valid.body,
        created_at: Utc::now(),
    };
    let payload = message_summary(&message);
    let state_for_insert = state.clone();
    let message_for_insert = message.clone();

    if let Err(error) = fanout_message_created(state, payload.clone()).await {
        error!(
            message_id = %message.id,
            server_id = %message.server_id,
            room_id = %message.room_id,
            user_id = %message.author_user_id,
            %error,
            "failed to schedule text chat fanout"
        );
    }

    tokio::spawn(async move {
        if let Err(error) = state_for_insert
            .text_chat_store
            .insert_text_message(message_for_insert.clone())
            .await
        {
            error!(
                message_id = %message_for_insert.id,
                server_id = %message_for_insert.server_id,
                room_id = %message_for_insert.room_id,
                user_id = %message_for_insert.author_user_id,
                %error,
                "failed to persist fanned out text chat message"
            );
        }
    });

    Ok(SendMessageAccepted { message: payload })
}

/// Text chat application error.
#[derive(Debug)]
pub(crate) enum TextChatApplicationError {
    /// Request shape or target is invalid.
    BadRequest(String),
    /// User cannot access the requested chat resource.
    Unauthorized(String),
    /// Resource was not found.
    NotFound(String),
    /// Unexpected internal failure.
    Internal(anyhow::Error),
}

async fn ensure_room_text_available(
    state: &AppState,
    user_id: &Uuid,
    server_id: &Uuid,
    room_id: &Uuid,
) -> Result<(), TextChatApplicationError> {
    let Some(room) = state
        .server_store
        .find_server_room(server_id, room_id)
        .await
        .map_err(TextChatApplicationError::Internal)?
    else {
        return Err(TextChatApplicationError::NotFound(
            "Комната не найдена.".to_owned(),
        ));
    };
    if room.kind == ServerRoomKind::Voice {
        return Err(TextChatApplicationError::BadRequest(
            "В этой комнате нет текстового чата.".to_owned(),
        ));
    }
    if policy::can_receive_room_event(state, user_id, server_id, room_id)
        .await
        .map_err(TextChatApplicationError::Internal)?
    {
        Ok(())
    } else {
        Err(TextChatApplicationError::Unauthorized(
            "Нет доступа к этой комнате.".to_owned(),
        ))
    }
}

async fn fanout_message_created(state: &AppState, message: TextChatMessage) -> anyhow::Result<()> {
    let server_id = Uuid::parse_str(&message.server_id)?;
    let room_id = Uuid::parse_str(&message.room_id)?;
    let candidates = state
        .realtime_hub
        .recipients(state, RealtimeModule::TextChat, &server_id)
        .await;
    let mut stream_ids = Vec::new();

    for candidate in candidates {
        match policy::can_receive_room_event(state, &candidate.user_id, &server_id, &room_id).await
        {
            Ok(true) => stream_ids.push(candidate.stream_id),
            Ok(false) => {}
            Err(error) => {
                tracing::warn!(
                    stream_id = %candidate.stream_id,
                    user_id = %candidate.user_id,
                    %error,
                    "failed to evaluate text chat fanout policy"
                );
            }
        }
    }

    state
        .realtime_hub
        .fanout_to_streams(
            RealtimeModule::TextChat,
            &server_id,
            RealtimeKind::TextChat(TextChatKind::MessageCreated),
            &stream_ids,
            message,
        )
        .await;

    Ok(())
}

fn parse_id(value: &str, message: &str) -> Result<Uuid, TextChatApplicationError> {
    Uuid::parse_str(value).map_err(|_| TextChatApplicationError::BadRequest(message.to_owned()))
}

fn message_summary(message: &TextMessage) -> TextChatMessage {
    TextChatMessage {
        id: message.id.to_string(),
        server_id: message.server_id.to_string(),
        room_id: message.room_id.to_string(),
        author_user_id: message.author_user_id.to_string(),
        author_nickname: message.author_nickname.clone(),
        body: message.body.clone(),
        created_at: message.created_at.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cheenhub_contracts::realtime::{LoadRoomHistory, SendMessage};
    use cheenhub_contracts::rest::{RegisterRequest, ServerRoomKind};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use super::{load_room_history, send_message};
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
            server_store: Arc::new(InMemoryServerStore::default()),
            text_chat_store: Arc::new(InMemoryTextChatStore::default()),
            voice_presence_store: Arc::new(
                crate::features::voice_chat::infrastructure::InMemoryVoicePresenceStore::default(),
            ),
            realtime_hub: Arc::new(RealtimeHub::default()),
            auth_keys: AuthKeys::generate_for_tests(),
            access_token_lifetime_minutes: 15,
            refresh_token_lifetime_days: 30,
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
                    created_at: base_time + Duration::seconds(index),
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
                    created_at: base_time + Duration::seconds(index),
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
                created_at: Utc::now(),
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
}
