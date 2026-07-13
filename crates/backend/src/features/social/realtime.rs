//! Realtime-адаптер друзей и личных сообщений.

use cheenhub_contracts::realtime::{
    ConversationReadCheckpoint as ReadCheckpointPayload, DirectMessageCreated, RealtimeEnvelope,
    RealtimeKind, RealtimeModule, RejectionCode, SocialChangeReason, SocialChanged, SocialKind,
    SocialReady,
};
use uuid::Uuid;

use crate::features::social::domain::ConversationReadCheckpoint;
use crate::realtime::EnvelopeSink;
use crate::realtime::protocol::{require_request_id, send_rejection, write_envelope};
use crate::state::AppState;

/// Обрабатывает realtime-сообщения social-модуля.
pub(crate) async fn handle(
    _state: &AppState,
    user_id: &Uuid,
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
