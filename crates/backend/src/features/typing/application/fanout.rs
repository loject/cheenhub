//! Рассылка событий набора участникам комнаты или личного диалога.

use cheenhub_contracts::realtime::{
    DirectMessageTypingChanged, RealtimeKind, RealtimeModule, SocialKind, TextChatKind,
    TypingAuthor, TypingChanged,
};
use uuid::Uuid;

use crate::features::social::{self, SocialError};
use crate::features::text_chat::policy;
use crate::features::typing::infrastructure::TypingAuthorEntry;
use crate::state::AppState;

pub(super) use crate::features::typing::infrastructure::TypingTargetKind;

/// Рассылает участникам комнаты событие о начале или завершении набора.
///
/// Автор события исключается из получателей: клиент и так знает о своем наборе,
/// а лишнее событие только обновляло бы пустое состояние.
pub(super) async fn notify_room_typing_changed(
    state: &AppState,
    server_id: &Uuid,
    room_id: &Uuid,
    entry: &TypingAuthorEntry,
    is_typing: bool,
) {
    let recipients = room_recipients(state, server_id, room_id, entry.user_id).await;
    tracing::debug!(
        %server_id, %room_id,
        user_id = %entry.user_id,
        is_typing,
        recipients = recipients.len(),
        "fanning out room typing event"
    );
    state
        .realtime_hub
        .fanout_to_streams(
            RealtimeModule::TextChat,
            server_id,
            RealtimeKind::TextChat(TextChatKind::TypingChanged),
            &recipients,
            TypingChanged {
                server_id: server_id.to_string(),
                room_id: room_id.to_string(),
                author: author_snapshot(entry),
                is_typing,
            },
        )
        .await;
}

/// Рассылает участникам диалога событие о начале или завершении набора.
pub(super) async fn notify_direct_message_typing_changed(
    state: &AppState,
    conversation_id: &Uuid,
    entry: &TypingAuthorEntry,
    is_typing: bool,
) {
    let participants = match conversation_participant_ids(state, conversation_id).await {
        Ok(participants) => participants,
        Err(error) => {
            tracing::warn!(
                %conversation_id, %error,
                "direct message typing recipients are unavailable"
            );
            return;
        }
    };
    let recipients = state
        .realtime_hub
        .recipients_for_users(RealtimeModule::Social, &participants)
        .await
        .into_iter()
        .filter(|recipient| recipient.user_id != entry.user_id)
        .map(|recipient| recipient.stream_id)
        .collect::<Vec<_>>();

    tracing::debug!(
        %conversation_id,
        user_id = %entry.user_id,
        is_typing,
        recipients = recipients.len(),
        "fanning out direct message typing event"
    );
    state
        .realtime_hub
        .fanout_to_streams(
            RealtimeModule::Social,
            conversation_id,
            RealtimeKind::Social(SocialKind::DirectMessageTypingChanged),
            &recipients,
            DirectMessageTypingChanged {
                conversation_id: conversation_id.to_string(),
                author: author_snapshot(entry),
                is_typing,
            },
        )
        .await;
}

/// Возвращает идентификаторы участников личного диалога.
///
/// Переиспользует проверку social-фичи: диалог доступен только при принятой
/// дружбе, поэтому посторонний пользователь не сможет ни начать набор, ни
/// подписаться на события чужого диалога.
pub(super) async fn conversation_participant_ids(
    state: &AppState,
    conversation_id: &Uuid,
) -> Result<Vec<Uuid>, anyhow::Error> {
    match social::direct_message_voice_user_ids(state, conversation_id).await {
        Ok(user_ids) => Ok(user_ids),
        Err(SocialError::Internal(error)) => Err(error),
        Err(error) => {
            tracing::warn!(%conversation_id, ?error, "direct message is not accessible");
            Ok(Vec::new())
        }
    }
}

/// Преобразует запись состояния набора в публичный снимок для клиента.
pub(super) fn author_snapshot(entry: &TypingAuthorEntry) -> TypingAuthor {
    TypingAuthor {
        user_id: entry.user_id.to_string(),
        nickname: entry.nickname.clone(),
        avatar_url: entry.avatar_url.clone(),
    }
}

async fn room_recipients(
    state: &AppState,
    server_id: &Uuid,
    room_id: &Uuid,
    exclude_user_id: Uuid,
) -> Vec<Uuid> {
    let candidates = state
        .realtime_hub
        .recipients(state, RealtimeModule::TextChat, server_id)
        .await;
    let mut stream_ids = Vec::new();

    for candidate in candidates {
        if candidate.user_id == exclude_user_id {
            continue;
        }
        match policy::can_receive_room_event(state, &candidate.user_id, server_id, room_id).await {
            Ok(true) => stream_ids.push(candidate.stream_id),
            Ok(false) => {}
            Err(error) => {
                tracing::warn!(
                    stream_id = %candidate.stream_id,
                    user_id = %candidate.user_id,
                    %server_id,
                    %room_id,
                    %error,
                    "failed to evaluate typing fanout recipient"
                );
            }
        }
    }

    stream_ids
}
