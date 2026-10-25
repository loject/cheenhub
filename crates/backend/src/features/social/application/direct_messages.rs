//! Сценарии приложения для личных сообщений.

use cheenhub_contracts::realtime::{DirectMessageCreated, SocialChangeReason};
use cheenhub_contracts::rest::{
    EditDmMessageRequest, EditDmMessageResponse, ListDmConversationsResponse,
    ListDmMessagesResponse, MarkDmConversationReadRequest, MarkDmConversationReadResponse,
    OpenDmConversationRequest, OpenDmConversationResponse, SendDmMessageRequest,
    SendDmMessageResponse,
};
use chrono::Utc;
use uuid::Uuid;

use super::attachments::validate_attachment_owner;
use crate::features::push_notifications::{DirectMessagePush, direct_message_preview};
use crate::features::social::domain::DmMessage;
use crate::features::social::error::SocialError;
use crate::features::social::infrastructure::normalize_unread_count;
use crate::features::social::realtime::{
    notify_conversation_read_checkpoint, notify_direct_message_created, notify_social_changed,
};
use crate::features::social::support::{
    conversation_summaries, conversation_summary, ensure_user_active, ensure_user_exists,
    load_user_conversation, message_body, message_summaries, message_summary, other_user_id,
    parse_id,
};
use crate::state::AppState;

/// Возвращает личные диалоги.
///
/// Вызывающий транспорт обязан проверить сессию и активность аккаунта `user_id`.
pub(crate) async fn list_dm_conversations(
    state: &AppState,
    user_id: &Uuid,
) -> Result<ListDmConversationsResponse, SocialError> {
    let conversations = state
        .social_store
        .conversations_for_user(user_id)
        .await
        .map_err(SocialError::Internal)?;
    Ok(ListDmConversationsResponse {
        conversations: conversation_summaries(state, user_id, conversations).await?,
    })
}

/// Открывает личный диалог с другим активным пользователем независимо от дружбы.
///
/// Вызывающий транспорт обязан проверить сессию и активность аккаунта `user_id`.
pub(crate) async fn open_dm_conversation(
    state: &AppState,
    user_id: &Uuid,
    request: OpenDmConversationRequest,
) -> Result<OpenDmConversationResponse, SocialError> {
    let friend_user_id = parse_id(&request.friend_user_id, "Пользователь не найден.")?;
    ensure_user_active(state, &friend_user_id).await?;
    if *user_id == friend_user_id {
        tracing::warn!(%user_id, "rejected direct dialog with self");
        return Err(SocialError::BadRequest(
            "Нельзя открыть личный диалог с собой.".to_owned(),
        ));
    }
    let conversation = state
        .social_store
        .get_or_create_conversation(user_id, &friend_user_id, Utc::now())
        .await
        .map_err(SocialError::Internal)?;
    tracing::info!(%user_id, peer_user_id = %friend_user_id, conversation_id = %conversation.id, "opened direct message dialog");
    notify_social_changed(
        state,
        &[*user_id, friend_user_id],
        SocialChangeReason::DirectMessages,
        Some(conversation.id),
    )
    .await;
    Ok(OpenDmConversationResponse {
        conversation: conversation_summary(state, user_id, conversation).await?,
    })
}

/// Возвращает страницу сообщений личного диалога.
///
/// Вызывающий транспорт обязан проверить сессию и активность аккаунта `user_id`.
pub(crate) async fn list_dm_messages(
    state: &AppState,
    user_id: &Uuid,
    conversation_id: String,
    before_message_id: Option<String>,
) -> Result<ListDmMessagesResponse, SocialError> {
    let conversation_id = parse_id(&conversation_id, "Диалог не найден.")?;
    let conversation = load_user_conversation(state, &conversation_id, user_id).await?;
    let friend_user_id = other_user_id(&conversation, user_id);
    let before_message_id = before_message_id
        .as_deref()
        .map(|value| parse_id(value, "История сообщений недоступна."))
        .transpose()?;
    let page = state
        .social_store
        .dm_message_page(&conversation.id, before_message_id.as_ref())
        .await
        .map_err(|error| {
            if before_message_id.is_some() {
                SocialError::BadRequest("История сообщений недоступна.".to_owned())
            } else {
                SocialError::Internal(error)
            }
        })?;
    let recipient_last_read_seq =
        recipient_last_read_seq(state, &conversation.id, &friend_user_id).await?;
    Ok(ListDmMessagesResponse {
        messages: message_summaries(state, user_id, recipient_last_read_seq, page.messages).await?,
        recipient_last_read_seq,
        has_more: page.has_more,
    })
}

/// Помечает личный диалог прочитанным до указанного сообщения.
///
/// Вызывающий транспорт обязан проверить сессию и активность аккаунта `user_id`.
pub(crate) async fn mark_dm_conversation_read(
    state: &AppState,
    user_id: &Uuid,
    conversation_id: String,
    request: MarkDmConversationReadRequest,
) -> Result<MarkDmConversationReadResponse, SocialError> {
    let conversation_id = parse_id(&conversation_id, "Диалог не найден.")?;
    let conversation = load_user_conversation(state, &conversation_id, user_id).await?;
    let last_read_message_id = parse_id(
        &request.last_read_message_id,
        "Сообщение не найдено в этом диалоге.",
    )?;
    let friend_user_id = other_user_id(&conversation, user_id);
    if state
        .social_store
        .dm_message_by_id(&conversation.id, &last_read_message_id)
        .await
        .map_err(SocialError::Internal)?
        .is_none()
    {
        tracing::warn!(
            conversation_id = %conversation.id,
            user_id = %user_id,
            %last_read_message_id,
            "rejected read checkpoint for message outside direct conversation"
        );
        return Err(SocialError::BadRequest(
            "Сообщение не найдено в этом диалоге.".to_owned(),
        ));
    }
    let now = Utc::now();
    let update = state
        .social_store
        .mark_conversation_read(&conversation.id, user_id, &last_read_message_id, now)
        .await
        .map_err(SocialError::Internal)?;
    if let Some(checkpoint) = update.checkpoint.clone() {
        tracing::info!(
            conversation_id = %conversation.id,
            user_id = %user_id,
            last_read_seq = checkpoint.last_read_seq,
            unread_count = normalize_unread_count(update.state.unread_count),
            "marked direct conversation read"
        );
        notify_conversation_read_checkpoint(state, &[friend_user_id], &checkpoint).await;
        notify_social_changed(
            state,
            &[*user_id, friend_user_id],
            SocialChangeReason::DirectMessages,
            Some(conversation.id),
        )
        .await;
    } else {
        tracing::debug!(
            conversation_id = %conversation.id,
            user_id = %user_id,
            last_read_message_id = %last_read_message_id,
            "ignored stale direct conversation read checkpoint"
        );
    }
    let total_unread_count = state
        .social_store
        .total_unread_count(user_id)
        .await
        .map_err(SocialError::Internal)?;
    Ok(MarkDmConversationReadResponse {
        conversation_id: conversation.id.to_string(),
        last_read_message_id: update
            .state
            .last_read_message_id
            .map(|message_id| message_id.to_string()),
        last_read_seq: update.state.last_read_seq,
        last_read_at: update
            .state
            .last_read_at
            .map(|read_at| read_at.to_rfc3339()),
        conversation_unread_count: normalize_unread_count(update.state.unread_count),
        total_unread_count,
    })
}

/// Отправляет личное сообщение.
///
/// Вызывающий транспорт обязан проверить сессию и активность аккаунта `user_id`.
pub(crate) async fn send_dm_message(
    state: &AppState,
    user_id: &Uuid,
    conversation_id: String,
    request: SendDmMessageRequest,
) -> Result<SendDmMessageResponse, SocialError> {
    let conversation_id = parse_id(&conversation_id, "Диалог не найден.")?;
    let conversation = load_user_conversation(state, &conversation_id, user_id).await?;
    let friend_user_id = other_user_id(&conversation, user_id);
    ensure_user_active(state, &friend_user_id).await?;
    let image_id = request
        .image_id
        .as_deref()
        .map(|value| parse_id(value, "Изображение недоступно."))
        .transpose()?;
    if let Some(image_id) = image_id {
        validate_attachment_owner(state, conversation_id, image_id, *user_id).await?;
    }
    let body = if request.body.trim().is_empty() && image_id.is_some() {
        String::new()
    } else {
        message_body(request.body)?
    };
    let current_user = ensure_user_exists(state, user_id).await?;
    let now = Utc::now();
    let message = state
        .social_store
        .insert_dm_message(DmMessage {
            id: Uuid::new_v4(),
            conversation_id,
            seq: 0,
            sender_user_id: *user_id,
            body,
            image_id,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        })
        .await
        .map_err(SocialError::Internal)?;
    tracing::info!(
        conversation_id = %conversation_id,
        message_id = %message.id,
        sender_user_id = %user_id,
        seq = message.seq,
        "sent direct message"
    );
    let message_preview = direct_message_preview(&message.body, message.image_id.is_some());
    let push_payload = DirectMessagePush::new(
        message.id,
        message.conversation_id,
        message.seq,
        *user_id,
        &current_user.nickname,
        &message_preview,
        message.created_at,
    );
    match state
        .push_notifications
        .enqueue_direct_message(friend_user_id, push_payload)
        .await
    {
        Ok(enqueued) => tracing::debug!(
            message_id = %message.id,
            recipient_user_id = %friend_user_id,
            installation_count = enqueued,
            "queued direct message push deliveries"
        ),
        Err(error) => tracing::error!(
            %error,
            message_id = %message.id,
            recipient_user_id = %friend_user_id,
            "failed to queue direct message push deliveries"
        ),
    }
    notify_direct_message_created(
        state,
        friend_user_id,
        DirectMessageCreated {
            message_id: message.id.to_string(),
            conversation_id: message.conversation_id.to_string(),
            message_seq: message.seq,
            sender_user_id: user_id.to_string(),
            sender_nickname: current_user.nickname.clone(),
            body: message_preview,
            created_at: message.created_at.to_rfc3339(),
        },
    )
    .await;
    notify_social_changed(
        state,
        &[*user_id, friend_user_id],
        SocialChangeReason::DirectMessages,
        Some(conversation_id),
    )
    .await;
    Ok(SendDmMessageResponse {
        message: message_summary(
            state,
            user_id,
            recipient_last_read_seq(state, &conversation_id, &friend_user_id).await?,
            message,
        )
        .await?,
    })
}

/// Изменяет только текст собственного неудалённого сообщения без ограничения по времени.
///
/// Вызывающий транспорт обязан проверить сессию и активность аккаунта `user_id`.
pub(crate) async fn edit_dm_message(
    state: &AppState,
    user_id: &Uuid,
    conversation_id: String,
    message_id: String,
    request: EditDmMessageRequest,
) -> Result<EditDmMessageResponse, SocialError> {
    let conversation_id = parse_id(&conversation_id, "Диалог не найден.")?;
    let conversation = load_user_conversation(state, &conversation_id, user_id).await?;
    let message_id = parse_id(&message_id, "Сообщение не найдено.")?;
    let original = state
        .social_store
        .dm_message_by_id(&conversation_id, &message_id)
        .await
        .map_err(SocialError::Internal)?
        .ok_or_else(|| SocialError::NotFound("Сообщение не найдено.".to_owned()))?;

    if original.sender_user_id != *user_id || original.deleted_at.is_some() {
        tracing::warn!(
            conversation_id = %conversation_id,
            message_id = %message_id,
            user_id = %user_id,
            "rejected direct message edit for non-author or deleted message"
        );
        return Err(SocialError::NotFound("Сообщение не найдено.".to_owned()));
    }

    let body = if request.body.trim().is_empty() && original.image_id.is_some() {
        String::new()
    } else {
        message_body(request.body)?
    };
    let changed = original.body != body;
    let updated = if changed {
        state
            .social_store
            .update_dm_message_body(&conversation_id, &message_id, user_id, &body, Utc::now())
            .await
            .map_err(SocialError::Internal)?
            .ok_or_else(|| SocialError::NotFound("Сообщение не найдено.".to_owned()))?
    } else {
        original
    };

    if changed {
        tracing::debug!(
            conversation_id = %conversation_id,
            message_id = %message_id,
            user_id = %user_id,
            "edited direct message"
        );
        let recipient_id = other_user_id(&conversation, user_id);
        notify_social_changed(
            state,
            &[*user_id, recipient_id],
            SocialChangeReason::DirectMessages,
            Some(conversation_id),
        )
        .await;
    }

    Ok(EditDmMessageResponse {
        message: message_summary(
            state,
            user_id,
            recipient_last_read_seq(
                state,
                &conversation_id,
                &other_user_id(&conversation, user_id),
            )
            .await?,
            updated,
        )
        .await?,
    })
}

async fn recipient_last_read_seq(
    state: &AppState,
    conversation_id: &Uuid,
    recipient_user_id: &Uuid,
) -> Result<i64, SocialError> {
    Ok(state
        .social_store
        .conversation_member_state(conversation_id, recipient_user_id)
        .await
        .map_err(SocialError::Internal)?
        .map(|state| state.last_read_seq)
        .unwrap_or(0))
}
