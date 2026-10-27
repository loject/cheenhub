//! Прикладные операции закрепления сообщений и уведомления участников комнаты.

use cheenhub_contracts::realtime::{
    ListPinnedMessages, PinMessage, PinMessageAccepted, PinnedMessages, PinnedTextMessage,
    RealtimeKind, RealtimeModule, TextChatKind, UnpinMessage, UnpinMessageAccepted,
};
use serde::Serialize;
use uuid::Uuid;

use super::{
    TextChatApplicationError, ensure_room_text_available, history_authors, message_summary,
    parse_id,
};
use crate::features::images::application as image_application;
use crate::features::text_chat::domain::PinnedMessage;
use crate::features::text_chat::infrastructure::{PIN_LIMIT, PinMessageResult};
use crate::features::text_chat::policy;
use crate::state::AppState;

/// Загружает закреплённые сообщения доступной пользователю комнаты.
pub(crate) async fn list_pinned_messages(
    state: &AppState,
    user_id: &Uuid,
    request: ListPinnedMessages,
) -> Result<PinnedMessages, TextChatApplicationError> {
    let (server_id, room_id) =
        parse_target(state, user_id, &request.server_id, &request.room_id).await?;
    let pins = state
        .text_chat_store
        .list_pinned_messages(&server_id, &room_id)
        .await
        .map_err(TextChatApplicationError::Internal)?;
    let (messages, deleted_authors) = summaries(state, &pins).await?;
    let author_ids = pins
        .iter()
        .filter(|pin| !deleted_authors[&pin.message.author_user_id])
        .map(|pin| pin.message.author_user_id)
        .collect::<Vec<_>>();
    let author_role_ids = if author_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        let server = state
            .server_store
            .find_server(&server_id)
            .await
            .map_err(TextChatApplicationError::Internal)?
            .ok_or_else(|| TextChatApplicationError::NotFound("Сервер не найден.".to_owned()))?;
        super::author_roles::author_role_ids(state, &server, &author_ids).await?
    };
    tracing::debug!(%server_id, %room_id, count = messages.len(), "listed pinned room messages");
    Ok(PinnedMessages {
        server_id: request.server_id,
        room_id: request.room_id,
        messages,
        limit: PIN_LIMIT as u32,
        author_role_ids,
    })
}

/// Закрепляет сохранённое сообщение, если у пользователя есть право модерации.
pub(crate) async fn pin_message(
    state: &AppState,
    user_id: &Uuid,
    request: PinMessage,
) -> Result<PinMessageAccepted, TextChatApplicationError> {
    let (server_id, room_id) =
        parse_target(state, user_id, &request.server_id, &request.room_id).await?;
    require_pin_permission(state, user_id, &server_id).await?;
    let message_id = parse_id(&request.message_id, "Сообщение не найдено.")?;
    let mut result = PinMessageResult::NotFound;
    // Отправка чата пока сохраняется в фоне; кратко повторяем поиск нового сообщения.
    for attempt in 0..3 {
        result = state
            .text_chat_store
            .pin_message(&server_id, &room_id, &message_id, user_id)
            .await
            .map_err(TextChatApplicationError::Internal)?;
        if !matches!(&result, PinMessageResult::NotFound) || attempt == 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    }
    let pin = match result {
        PinMessageResult::Pinned(pin) => *pin,
        PinMessageResult::AlreadyPinned => {
            return Err(TextChatApplicationError::BadRequest(
                "Сообщение уже закреплено.".to_owned(),
            ));
        }
        PinMessageResult::LimitReached => {
            return Err(TextChatApplicationError::BadRequest(
                "Достигнут лимит закреплённых сообщений в комнате.".to_owned(),
            ));
        }
        PinMessageResult::NotFound => {
            return Err(TextChatApplicationError::NotFound(
                "Сообщение не найдено или уже удалено.".to_owned(),
            ));
        }
    };
    let (summarized, _) = summaries(state, &[pin]).await?;
    let pinned = summarized.into_iter().next().ok_or_else(|| {
        TextChatApplicationError::Internal(anyhow::anyhow!("saved message pin vanished"))
    })?;
    tracing::debug!(%server_id, %room_id, %message_id, %user_id, "pinned room message");
    if let Err(error) = fanout_pin_change(
        state,
        &server_id,
        &room_id,
        TextChatKind::MessagePinned,
        pinned.clone(),
    )
    .await
    {
        tracing::warn!(%message_id, %error, "failed to fanout message pin");
    }
    Ok(PinMessageAccepted { pinned })
}

/// Снимает закрепление сообщения, сохраняя его в истории.
pub(crate) async fn unpin_message(
    state: &AppState,
    user_id: &Uuid,
    request: UnpinMessage,
) -> Result<UnpinMessageAccepted, TextChatApplicationError> {
    let (server_id, room_id) =
        parse_target(state, user_id, &request.server_id, &request.room_id).await?;
    require_pin_permission(state, user_id, &server_id).await?;
    let message_id = parse_id(&request.message_id, "Сообщение не найдено.")?;
    let removed = state
        .text_chat_store
        .unpin_message(&server_id, &room_id, &message_id)
        .await
        .map_err(TextChatApplicationError::Internal)?;
    if !removed {
        return Err(TextChatApplicationError::NotFound(
            "Закрепление не найдено.".to_owned(),
        ));
    }
    let payload = UnpinMessageAccepted {
        server_id: server_id.to_string(),
        room_id: room_id.to_string(),
        message_id: message_id.to_string(),
    };
    tracing::debug!(%server_id, %room_id, %message_id, %user_id, "unpinned room message");
    if let Err(error) = fanout_pin_change(
        state,
        &server_id,
        &room_id,
        TextChatKind::MessageUnpinned,
        payload.clone(),
    )
    .await
    {
        tracing::warn!(%message_id, %error, "failed to fanout message unpin");
    }
    Ok(payload)
}

async fn parse_target(
    state: &AppState,
    user_id: &Uuid,
    server_id: &str,
    room_id: &str,
) -> Result<(Uuid, Uuid), TextChatApplicationError> {
    let server_id = parse_id(server_id, "Сервер не найден.")?;
    let room_id = parse_id(room_id, "Комната не найдена.")?;
    ensure_room_text_available(state, user_id, &server_id, &room_id).await?;
    Ok((server_id, room_id))
}

async fn require_pin_permission(
    state: &AppState,
    user_id: &Uuid,
    server_id: &Uuid,
) -> Result<(), TextChatApplicationError> {
    if policy::can_pin_messages(state, user_id, server_id)
        .await
        .map_err(TextChatApplicationError::Internal)?
    {
        return Ok(());
    }
    tracing::warn!(%server_id, %user_id, "rejected unauthorized message pin action");
    Err(TextChatApplicationError::Unauthorized(
        "Недостаточно прав для управления закреплёнными сообщениями.".to_owned(),
    ))
}

async fn summaries(
    state: &AppState,
    pins: &[PinnedMessage],
) -> Result<
    (
        Vec<PinnedTextMessage>,
        std::collections::HashMap<Uuid, bool>,
    ),
    TextChatApplicationError,
> {
    let avatars = image_application::avatar_urls_by_user_ids(
        state,
        pins.iter().map(|pin| pin.message.author_user_id),
    )
    .await
    .map_err(TextChatApplicationError::Internal)?;
    let messages = pins
        .iter()
        .map(|pin| pin.message.clone())
        .collect::<Vec<_>>();
    let deleted = history_authors::deleted_authors(state, &messages).await?;
    let summaries = pins
        .iter()
        .map(|pin| {
            let mut message = message_summary(
                &pin.message,
                avatars.get(&pin.message.author_user_id).cloned(),
            );
            if deleted.get(&pin.message.author_user_id) == Some(&true) {
                message.author_nickname = "Удалённый пользователь".to_owned();
                message.author_avatar_url = None;
            }
            PinnedTextMessage {
                message,
                pinned_by_user_id: pin.pinned_by_user_id.to_string(),
                pinned_at: pin.pinned_at.to_rfc3339(),
            }
        })
        .collect::<Vec<_>>();
    Ok((summaries, deleted))
}

async fn fanout_pin_change<T: Serialize>(
    state: &AppState,
    server_id: &Uuid,
    room_id: &Uuid,
    kind: TextChatKind,
    payload: T,
) -> anyhow::Result<()> {
    let candidates = crate::features::servers::realtime::server_recipients(
        state,
        RealtimeModule::TextChat,
        server_id,
    )
    .await;
    let mut stream_ids = Vec::new();
    for candidate in candidates {
        match policy::can_receive_room_event(state, &candidate.user_id, server_id, room_id).await {
            Ok(true) => stream_ids.push(candidate.stream_id),
            Ok(false) => {}
            Err(error) => tracing::warn!(
                stream_id = %candidate.stream_id,
                user_id = %candidate.user_id,
                %error,
                "failed to evaluate message pin fanout policy"
            ),
        }
    }
    state
        .realtime_hub
        .fanout_to_streams(
            RealtimeModule::TextChat,
            server_id,
            RealtimeKind::TextChat(kind),
            &stream_ids,
            payload,
        )
        .await;
    Ok(())
}
