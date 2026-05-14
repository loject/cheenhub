//! Text chat application flows.

use cheenhub_contracts::realtime::{
    LoadRoomHistory, RealtimeKind, RealtimeModule, RoomHistory, SendMessage, SendMessageAccepted,
    TextChatKind, TextChatMessage,
};
use cheenhub_contracts::rest::AuthUser;
use cheenhub_contracts::rest::ServerRoomKind;
use chrono::Utc;
use tracing::error;
use uuid::Uuid;

use crate::features::images::application as image_application;
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

    let avatar_urls = image_application::avatar_urls_by_user_ids(
        state,
        page.messages.iter().map(|message| message.author_user_id),
    )
    .await
    .map_err(TextChatApplicationError::Internal)?;

    Ok(RoomHistory {
        server_id: server_id.to_string(),
        room_id: room_id.to_string(),
        messages: page
            .messages
            .iter()
            .map(|message| {
                message_summary(message, avatar_urls.get(&message.author_user_id).cloned())
            })
            .collect(),
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
    let payload = message_summary(&message, user.avatar_url.clone());
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

fn message_summary(message: &TextMessage, author_avatar_url: Option<String>) -> TextChatMessage {
    TextChatMessage {
        id: message.id.to_string(),
        server_id: message.server_id.to_string(),
        room_id: message.room_id.to_string(),
        author_user_id: message.author_user_id.to_string(),
        author_nickname: message.author_nickname.clone(),
        author_avatar_url,
        body: message.body.clone(),
        created_at: message.created_at.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests;
