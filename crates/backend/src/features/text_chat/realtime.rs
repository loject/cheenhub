//! Text chat realtime adapter.

use cheenhub_contracts::realtime::{
    DeleteMessage, LoadRoomHistory, RealtimeEnvelope, RealtimeKind, RealtimeModule, RejectionCode,
    SendMessage, TextChatKind,
};
use cheenhub_contracts::rest::AuthUser;
use uuid::Uuid;

use crate::features::text_chat::application::{self, TextChatApplicationError};
use crate::realtime::EnvelopeSink;
use crate::realtime::protocol::{
    decode_payload, require_request_id, send_rejection, write_envelope,
};
use crate::state::AppState;

/// Handles one text chat module envelope.
pub(crate) async fn handle(
    state: &AppState,
    user: &AuthUser,
    user_id: &Uuid,
    send: &EnvelopeSink,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<()> {
    match envelope.kind {
        RealtimeKind::TextChat(TextChatKind::LoadRoomHistory) => {
            let request_id = require_request_id(&envelope)?;
            let payload: LoadRoomHistory = decode_payload(&envelope)?;
            match application::load_room_history(state, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::TextChat,
                        RealtimeKind::TextChat(TextChatKind::RoomHistory),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_application_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::TextChat(TextChatKind::SendMessage) => {
            let request_id = require_request_id(&envelope)?;
            let payload: SendMessage = decode_payload(&envelope)?;
            match application::send_message(state, user, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::TextChat,
                        RealtimeKind::TextChat(TextChatKind::SendMessageAccepted),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_application_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::TextChat(TextChatKind::DeleteMessage) => {
            let request_id = require_request_id(&envelope)?;
            let payload: DeleteMessage = decode_payload(&envelope)?;
            match application::delete_message(state, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::TextChat,
                        RealtimeKind::TextChat(TextChatKind::DeleteMessageAccepted),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_application_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::TextChat(_) => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::UnsupportedMessage,
                "Unsupported text chat message.",
            )
            .await
        }
        _ => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::BadRequest,
                "Realtime kind does not belong to text chat module.",
            )
            .await
        }
    }
}

async fn reject_application_error(
    send: &EnvelopeSink,
    request_id: Option<Uuid>,
    error: TextChatApplicationError,
) -> anyhow::Result<()> {
    match error {
        TextChatApplicationError::BadRequest(message) => {
            send_rejection(send, request_id, RejectionCode::BadRequest, &message).await
        }
        TextChatApplicationError::Unauthorized(message) => {
            send_rejection(send, request_id, RejectionCode::Unauthorized, &message).await
        }
        TextChatApplicationError::NotFound(message) => {
            send_rejection(send, request_id, RejectionCode::BadRequest, &message).await
        }
        TextChatApplicationError::Internal(error) => {
            tracing::error!(%error, "text chat realtime request failed");
            send_rejection(
                send,
                request_id,
                RejectionCode::InternalError,
                "Не удалось выполнить действие чата.",
            )
            .await
        }
    }
}
