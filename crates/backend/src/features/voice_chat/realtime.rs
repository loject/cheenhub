//! Voice chat realtime adapter.

use cheenhub_contracts::realtime::{
    JoinVoiceRoom, KickVoiceMember, LeaveVoiceRoom, RealtimeEnvelope, RealtimeKind, RealtimeModule,
    RejectionCode, VoiceChatKind,
};
use cheenhub_contracts::rest::AuthUser;
use uuid::Uuid;

use crate::features::voice_chat::application::{self, VoiceChatApplicationError};
use crate::realtime::EnvelopeSink;
use crate::realtime::protocol::{
    decode_payload, require_request_id, send_rejection, write_envelope,
};
use crate::state::AppState;

/// Handles one voice chat module envelope.
pub(crate) async fn handle(
    state: &AppState,
    user: &AuthUser,
    user_id: &Uuid,
    realtime_stream_id: Uuid,
    session_id: Uuid,
    send: &EnvelopeSink,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<()> {
    match envelope.kind {
        RealtimeKind::VoiceChat(VoiceChatKind::JoinVoiceRoom) => {
            let request_id = require_request_id(&envelope)?;
            let payload: JoinVoiceRoom = decode_payload(&envelope)?;
            match application::join_room(
                state,
                realtime_stream_id,
                session_id,
                user,
                user_id,
                payload,
            )
            .await
            {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::VoiceChat,
                        RealtimeKind::VoiceChat(VoiceChatKind::VoiceRoomSnapshot),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_application_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::VoiceChat(VoiceChatKind::LeaveVoiceRoom) => {
            let request_id = require_request_id(&envelope)?;
            let payload: LeaveVoiceRoom = decode_payload(&envelope)?;
            match application::leave_room(state, realtime_stream_id, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::VoiceChat,
                        RealtimeKind::VoiceChat(VoiceChatKind::VoiceRoomSnapshot),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_application_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::VoiceChat(VoiceChatKind::KickVoiceMember) => {
            let request_id = require_request_id(&envelope)?;
            let payload: KickVoiceMember = decode_payload(&envelope)?;
            match application::kick_member(state, user_id, payload).await {
                Ok(response) => {
                    write_envelope(
                        send,
                        RealtimeModule::VoiceChat,
                        RealtimeKind::VoiceChat(VoiceChatKind::VoiceRoomSnapshot),
                        Some(request_id),
                        response,
                    )
                    .await
                }
                Err(error) => reject_application_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::VoiceChat(_) => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::UnsupportedMessage,
                "Unsupported voice chat message.",
            )
            .await
        }
        _ => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::BadRequest,
                "Realtime kind does not belong to voice chat module.",
            )
            .await
        }
    }
}

async fn reject_application_error(
    send: &EnvelopeSink,
    request_id: Option<Uuid>,
    error: VoiceChatApplicationError,
) -> anyhow::Result<()> {
    match error {
        VoiceChatApplicationError::BadRequest(message) => {
            send_rejection(send, request_id, RejectionCode::BadRequest, &message).await
        }
        VoiceChatApplicationError::Unauthorized(message) => {
            send_rejection(send, request_id, RejectionCode::Unauthorized, &message).await
        }
        VoiceChatApplicationError::NotFound(message) => {
            send_rejection(send, request_id, RejectionCode::BadRequest, &message).await
        }
        VoiceChatApplicationError::Internal(error) => {
            tracing::error!(%error, "voice chat realtime request failed");
            send_rejection(
                send,
                request_id,
                RejectionCode::InternalError,
                "Не удалось выполнить действие голосовой комнаты.",
            )
            .await
        }
    }
}
