//! Realtime-адаптер чтения объединённого списка друзей и собеседников.
//!
//! Пользователь определяется аутентифицированной сессией, ошибки запроса
//! возвращаются как отказ без завершения исправного realtime-потока.

use cheenhub_contracts::realtime::{
    ListSocialEntries, RealtimeEnvelope, RealtimeKind, RealtimeModule, RejectionCode, SocialKind,
};
use uuid::Uuid;

use crate::features::social::{application, error::SocialError};
use crate::realtime::EnvelopeSink;
use crate::realtime::protocol::{send_rejection, write_envelope};
use crate::state::AppState;

/// Отвечает страницей объединённого списка на запрос проверенного пользователя.
pub(super) async fn handle(
    state: &AppState,
    user_id: &Uuid,
    send: &EnvelopeSink,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<()> {
    let Some(request_id) = envelope.request_id else {
        tracing::warn!(%user_id, "rejected social list request without request_id");
        return send_rejection(
            send,
            None,
            RejectionCode::BadRequest,
            "Для запроса списка нужен request_id.",
        )
        .await;
    };
    let request: ListSocialEntries = match serde_json::from_value(envelope.payload) {
        Ok(request) => request,
        Err(_) => {
            tracing::warn!(%user_id, %request_id, "rejected malformed social list request");
            return send_rejection(
                send,
                Some(request_id),
                RejectionCode::BadRequest,
                "Не удалось прочитать параметры списка.",
            )
            .await;
        }
    };
    match application::list_social_entries(state, user_id, request).await {
        Ok(page) => {
            tracing::debug!(%user_id, %request_id, returned_count = page.entries.len(), has_more = page.has_more, "completed social list realtime request");
            write_envelope(
                send,
                RealtimeModule::Social,
                RealtimeKind::Social(SocialKind::SocialEntries),
                Some(request_id),
                page,
            )
            .await
        }
        Err(error) => reject(send, user_id, request_id, error).await,
    }
}

async fn reject(
    send: &EnvelopeSink,
    user_id: &Uuid,
    request_id: Uuid,
    error: SocialError,
) -> anyhow::Result<()> {
    let (code, message) = match error {
        SocialError::BadRequest(message)
        | SocialError::NotFound(message)
        | SocialError::Conflict(message) => (RejectionCode::BadRequest, message),
        SocialError::Unauthorized(message) => (RejectionCode::Unauthorized, message),
        SocialError::Internal(error) => {
            tracing::error!(%user_id, %request_id, %error, "failed to load social list");
            return send_rejection(
                send,
                Some(request_id),
                RejectionCode::InternalError,
                "Не удалось загрузить список друзей и диалогов.",
            )
            .await;
        }
    };
    tracing::warn!(%user_id, %request_id, ?code, "rejected social list request");
    send_rejection(send, Some(request_id), code, &message).await
}
