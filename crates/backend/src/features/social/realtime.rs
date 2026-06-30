//! Realtime-адаптер друзей и личных сообщений.

use cheenhub_contracts::realtime::{
    RealtimeEnvelope, RealtimeKind, RealtimeModule, RejectionCode, SocialChangeReason,
    SocialChanged, SocialKind, SocialReady,
};
use uuid::Uuid;

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
