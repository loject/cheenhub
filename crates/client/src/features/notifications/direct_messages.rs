//! Общая обработка изменений непрочитанных личных сообщений.

use cheenhub_contracts::realtime::SocialChanged;
use dioxus::prelude::*;

use crate::features::app::current_user::CurrentUserContext;
use crate::features::social::api;

/// Данные нового личного сообщения, достаточные для уведомления.
pub(super) struct DmNotificationData {
    /// Идентификатор диалога.
    pub(super) conversation_id: String,
    /// Никнейм отправителя.
    pub(super) sender_nickname: String,
    /// Текст сообщения.
    pub(super) body: String,
}

/// Извлекает данные нового личного сообщения из изменения social-состояния.
pub(super) async fn extract_notification(
    event: &SocialChanged,
    current_user: &CurrentUserContext,
    mut unread_snapshot: Signal<Vec<(String, i64)>>,
) -> Option<DmNotificationData> {
    let conversations = match api::list_dm_conversations().await {
        Ok(conversations) => conversations,
        Err(error) => {
            debug!(%error, "failed to load DM conversations for notification");
            return None;
        }
    };

    let previous_snapshot = unread_snapshot();
    let next_snapshot = conversations
        .iter()
        .map(|conversation| (conversation.id.clone(), conversation.unread_count))
        .collect::<Vec<_>>();
    let conversation = match event.conversation_id.as_ref() {
        Some(conversation_id) => conversations
            .iter()
            .find(|item| item.id == *conversation_id),
        None => conversations.iter().find(|item| item.unread_count > 0),
    }
    .cloned();

    let Some(conversation) = conversation else {
        unread_snapshot.set(next_snapshot);
        return None;
    };

    let previous_unread = unread_count_for(&previous_snapshot, &conversation.id);
    let unread_increased = previous_unread
        .map(|unread_count| conversation.unread_count > unread_count)
        .unwrap_or(false);
    unread_snapshot.set(next_snapshot);
    if !unread_increased {
        debug!(
            conversation_id = %conversation.id,
            unread_count = conversation.unread_count,
            previous_unread = previous_unread.unwrap_or_default(),
            "skipping direct message notification without unread increase"
        );
        return None;
    }

    let messages = match api::list_dm_messages(&conversation.id, None).await {
        Ok(response) => response.messages,
        Err(error) => {
            debug!(%error, conversation_id = %conversation.id, "failed to load DM messages for notification");
            return None;
        }
    };
    let current_user_id = current_user.require_user().id;
    let message = messages
        .into_iter()
        .rev()
        .find(|message| message.sender_user_id != current_user_id)?;

    Some(DmNotificationData {
        conversation_id: conversation.id,
        sender_nickname: message.sender_nickname,
        body: message.body,
    })
}

/// Загружает стартовый снимок непрочитанных личных сообщений.
pub(super) async fn load_initial_unread_snapshot(mut unread_snapshot: Signal<Vec<(String, i64)>>) {
    match api::list_dm_conversations().await {
        Ok(conversations) => {
            unread_snapshot.set(
                conversations
                    .into_iter()
                    .map(|conversation| (conversation.id, conversation.unread_count))
                    .collect(),
            );
            debug!("loaded initial direct message unread snapshot for notifications");
        }
        Err(error) => debug!(%error, "failed to load initial DM unread snapshot for notifications"),
    }
}

fn unread_count_for(snapshot: &[(String, i64)], conversation_id: &str) -> Option<i64> {
    snapshot
        .iter()
        .find_map(|(saved_conversation_id, unread_count)| {
            (saved_conversation_id == conversation_id).then_some(*unread_count)
        })
}
