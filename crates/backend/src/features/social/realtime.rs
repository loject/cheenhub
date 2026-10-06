//! Realtime-адаптер друзей и личных сообщений.

use anyhow::Context;

use cheenhub_contracts::realtime::{
    ConversationReadCheckpoint as ReadCheckpointPayload, DirectMessageCreated,
    DirectMessageTypingSnapshotRequest, RealtimeEnvelope, RealtimeKind, RealtimeModule,
    RejectionCode, SocialChangeReason, SocialChanged, SocialKind, SocialReady,
    StartDirectMessageTyping, StopDirectMessageTyping,
};
use cheenhub_contracts::rest::AuthUser;
use uuid::Uuid;

use crate::features::social::domain::ConversationReadCheckpoint;
use crate::features::typing::{self, TypingAuthorEntry, TypingTarget, send_typing_rejection};
use crate::realtime::EnvelopeSink;
use crate::realtime::protocol::{require_request_id, send_rejection, write_envelope};
use crate::state::AppState;

/// Обрабатывает realtime-сообщения social-модуля.
pub(crate) async fn handle(
    state: &AppState,
    user: &AuthUser,
    user_id: &Uuid,
    realtime_stream_id: Uuid,
    send: &EnvelopeSink,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<()> {
    match envelope.kind {
        RealtimeKind::Social(SocialKind::Subscribe) => {
            let request_id = require_request_id(&envelope)?;
            tracing::debug!(%user_id, "subscribed social realtime stream");
            write_envelope(
                send,
                RealtimeModule::Social,
                RealtimeKind::Social(SocialKind::Ready),
                Some(request_id),
                SocialReady,
            )
            .await
        }
        RealtimeKind::Social(SocialKind::StartDirectMessageTyping) => {
            let request_id = envelope.request_id;
            let payload: StartDirectMessageTyping = decode_typing_payload(&envelope)?;
            let conversation_id = match typing::parse_conversation_id(&payload.conversation_id) {
                Ok(conversation_id) => conversation_id,
                Err(error) => {
                    return send_typing_rejection(send, request_id, error).await;
                }
            };
            let entry = TypingAuthorEntry {
                target: TypingTarget::direct_message(conversation_id),
                user_id: *user_id,
                nickname: user.nickname.clone(),
                avatar_url: user.avatar_url.clone(),
                realtime_stream_id,
                refreshed_at: tokio::time::Instant::now(),
            };
            match typing::start_direct_message_typing(state, entry, conversation_id).await {
                Ok(()) => Ok(()),
                Err(error) => send_typing_rejection(send, request_id, error).await,
            }
        }
        RealtimeKind::Social(SocialKind::StopDirectMessageTyping) => {
            let request_id = envelope.request_id;
            let payload: StopDirectMessageTyping = decode_typing_payload(&envelope)?;
            let conversation_id = match typing::parse_conversation_id(&payload.conversation_id) {
                Ok(conversation_id) => conversation_id,
                Err(error) => {
                    return send_typing_rejection(send, request_id, error).await;
                }
            };
            let target = TypingTarget::direct_message(conversation_id);
            match typing::stop_direct_message_typing(state, target, *user_id, realtime_stream_id)
                .await
            {
                Ok(()) => Ok(()),
                Err(error) => send_typing_rejection(send, request_id, error).await,
            }
        }
        RealtimeKind::Social(SocialKind::DirectMessageTypingSnapshot) => {
            let request_id = require_request_id(&envelope)?;
            let payload: DirectMessageTypingSnapshotRequest = decode_typing_payload(&envelope)?;
            match typing::direct_message_typing_snapshot(state, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::Social,
                        RealtimeKind::Social(SocialKind::DirectMessageTypingSnapshot),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => send_typing_rejection(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::Social(_) => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::UnsupportedMessage,
                "Unsupported social realtime message.",
            )
            .await
        }
        _ => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::BadRequest,
                "Realtime kind does not belong to social module.",
            )
            .await
        }
    }
}

/// Разбирает полезную нагрузку операции набора в личном диалоге.
///
/// Ошибка декодирования не является ошибкой доступа, поэтому отправляется как
/// отказ формата, а не как отказ доступа к диалогу.
fn decode_typing_payload<T: serde::de::DeserializeOwned>(
    envelope: &RealtimeEnvelope,
) -> Result<T, anyhow::Error> {
    serde_json::from_value(envelope.payload.clone())
        .context("failed to decode direct message typing payload")
}

/// Отправляет получателю точные данные нового личного сообщения.
pub(crate) async fn notify_direct_message_created(
    state: &AppState,
    recipient_user_id: Uuid,
    payload: DirectMessageCreated,
) {
    tracing::debug!(
        recipient_user_id = %recipient_user_id,
        conversation_id = %payload.conversation_id,
        message_id = %payload.message_id,
        message_seq = payload.message_seq,
        "fanning out direct message created event"
    );
    state
        .realtime_hub
        .fanout_to_user_streams(
            RealtimeModule::Social,
            RealtimeKind::Social(SocialKind::DirectMessageCreated),
            &[recipient_user_id],
            payload,
        )
        .await;
}

/// Отправляет social-событие во все активные потоки указанных пользователей.
pub(crate) async fn notify_social_changed(
    state: &AppState,
    user_ids: &[Uuid],
    reason: SocialChangeReason,
    conversation_id: Option<Uuid>,
) {
    let mut recipients = Vec::new();
    for user_id in user_ids {
        if !recipients.contains(user_id) {
            recipients.push(*user_id);
        }
    }
    if recipients.is_empty() {
        return;
    }
    tracing::debug!(
        recipient_count = recipients.len(),
        ?reason,
        ?conversation_id,
        "fanning out social realtime change"
    );
    let conversation_id = conversation_id.map(|id| id.to_string());
    state
        .realtime_hub
        .fanout_to_user_streams(
            RealtimeModule::Social,
            RealtimeKind::Social(SocialKind::Changed),
            &recipients,
            SocialChanged {
                reason,
                conversation_id,
            },
        )
        .await;
}

/// Отправляет checkpoint прочтения участникам, которым нужен статус исходящих сообщений.
pub(crate) async fn notify_conversation_read_checkpoint(
    state: &AppState,
    user_ids: &[Uuid],
    checkpoint: &ConversationReadCheckpoint,
) {
    let mut recipients = Vec::new();
    for user_id in user_ids {
        if !recipients.contains(user_id) {
            recipients.push(*user_id);
        }
    }
    if recipients.is_empty() {
        return;
    }
    tracing::debug!(
        recipient_count = recipients.len(),
        checkpoint_id = %checkpoint.id,
        conversation_id = %checkpoint.conversation_id,
        reader_user_id = %checkpoint.user_id,
        last_read_seq = checkpoint.last_read_seq,
        created_at = %checkpoint.created_at,
        "fanning out direct conversation read checkpoint"
    );
    state
        .realtime_hub
        .fanout_to_user_streams(
            RealtimeModule::Social,
            RealtimeKind::Social(SocialKind::ConversationReadCheckpoint),
            &recipients,
            ReadCheckpointPayload {
                conversation_id: checkpoint.conversation_id.to_string(),
                reader_user_id: checkpoint.user_id.to_string(),
                last_read_message_id: checkpoint.last_read_message_id.to_string(),
                last_read_seq: checkpoint.last_read_seq,
                read_at: checkpoint.read_at.to_rfc3339(),
            },
        )
        .await;
}
