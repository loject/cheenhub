//! Text chat realtime adapter.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use cheenhub_contracts::realtime::{
    ChatImageLoadedResponse, DeleteMessage, LoadChatImage, LoadRoomHistory, RealtimeEnvelope,
    RealtimeKind, RealtimeModule, RejectionCode, SendMessage, TextChatKind, UploadChatImage,
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
        RealtimeKind::TextChat(TextChatKind::UploadImage) => {
            let request_id = require_request_id(&envelope)?;
            let payload: UploadChatImage = decode_payload(&envelope)?;
            let bytes = match BASE64.decode(payload.data_base64.as_bytes()) {
                Ok(bytes) => bytes,
                Err(error) => {
                    tracing::warn!(
                        request_id = %request_id,
                        user_id = %user_id,
                        %error,
                        "rejected malformed text chat image upload payload"
                    );
                    return send_rejection(
                        send,
                        Some(request_id),
                        RejectionCode::BadRequest,
                        "Не удалось прочитать изображение.",
                    )
                    .await;
                }
            };
            tracing::debug!(
                request_id = %request_id,
                user_id = %user_id,
                input_bytes = bytes.len(),
                "received text chat image upload over realtime"
            );
            match application::upload_chat_image(
                state,
                user_id,
                payload.server_id,
                payload.room_id,
                payload.original_filename,
                &bytes,
            )
            .await
            {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::TextChat,
                        RealtimeKind::TextChat(TextChatKind::UploadImageAccepted),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_application_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::TextChat(TextChatKind::LoadImage) => {
            let request_id = require_request_id(&envelope)?;
            let payload: LoadChatImage = decode_payload(&envelope)?;
            match application::chat_image(state, user_id, payload.attachment_id).await {
                Ok((attachment, bytes)) => {
                    tracing::debug!(
                        request_id = %request_id,
                        user_id = %user_id,
                        attachment_id = %attachment.id,
                        byte_size = bytes.len(),
                        "loaded text chat image over realtime"
                    );
                    write_envelope(
                        send,
                        RealtimeModule::TextChat,
                        RealtimeKind::TextChat(TextChatKind::ImageLoaded),
                        Some(request_id),
                        ChatImageLoadedResponse {
                            id: attachment.id.to_string(),
                            content_type: attachment.content_type,
                            data_base64: BASE64.encode(bytes),
                        },
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
        TextChatApplicationError::Misconfigured {
            feature,
            missing,
            message,
        } => {
            tracing::warn!(
                feature,
                missing_env = ?missing,
                "text chat realtime feature is not configured"
            );
            send_rejection(send, request_id, RejectionCode::InternalError, &message).await
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
