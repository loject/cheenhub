//! Realtime-команды приглашений в серверные голосовые комнаты.

use cheenhub_contracts::realtime::{
    CancelVoiceRoomInvite, ListVoiceRoomInviteCandidates, ListVoiceRoomInvites, RealtimeEnvelope,
    RealtimeKind, RealtimeModule, RespondVoiceRoomInvite, SendVoiceRoomInvite, VoiceChatKind,
};
use cheenhub_contracts::rest::AuthUser;
use uuid::Uuid;

use crate::features::voice_chat::application;
use crate::realtime::EnvelopeSink;
use crate::realtime::protocol::{decode_payload, require_request_id, write_envelope};
use crate::state::AppState;

use super::reject_application_error;

/// Выполняет одну команду приглашений, сохраняя request_id в ответе.
pub(super) async fn handle(
    state: &AppState,
    user: &AuthUser,
    user_id: &Uuid,
    realtime_stream_id: Uuid,
    send: &EnvelopeSink,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<()> {
    match envelope.kind {
        RealtimeKind::VoiceChat(VoiceChatKind::ListVoiceRoomInviteCandidates) => {
            let request_id = require_request_id(&envelope)?;
            let request: ListVoiceRoomInviteCandidates = decode_payload(&envelope)?;
            match application::list_voice_room_invite_candidates(
                state,
                realtime_stream_id,
                user_id,
                request,
            )
            .await
            {
                Ok(snapshot) => {
                    write_envelope(
                        send,
                        RealtimeModule::VoiceChat,
                        RealtimeKind::VoiceChat(VoiceChatKind::VoiceRoomInviteCandidatesSnapshot),
                        Some(request_id),
                        snapshot,
                    )
                    .await
                }
                Err(error) => reject_application_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::VoiceChat(VoiceChatKind::SendVoiceRoomInvite) => {
            let request_id = require_request_id(&envelope)?;
            let request: SendVoiceRoomInvite = decode_payload(&envelope)?;
            match application::send_voice_room_invite(
                state,
                realtime_stream_id,
                user,
                user_id,
                request,
            )
            .await
            {
                Ok(event) => {
                    write_envelope(
                        send,
                        RealtimeModule::VoiceChat,
                        RealtimeKind::VoiceChat(VoiceChatKind::VoiceRoomInviteLifecycleEvent),
                        Some(request_id),
                        event,
                    )
                    .await
                }
                Err(error) => reject_application_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::VoiceChat(VoiceChatKind::RespondVoiceRoomInvite) => {
            let request_id = require_request_id(&envelope)?;
            let request: RespondVoiceRoomInvite = decode_payload(&envelope)?;
            match application::respond_voice_room_invite(state, user_id, request).await {
                Ok(event) => {
                    write_envelope(
                        send,
                        RealtimeModule::VoiceChat,
                        RealtimeKind::VoiceChat(VoiceChatKind::VoiceRoomInviteLifecycleEvent),
                        Some(request_id),
                        event,
                    )
                    .await
                }
                Err(error) => reject_application_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::VoiceChat(VoiceChatKind::CancelVoiceRoomInvite) => {
            let request_id = require_request_id(&envelope)?;
            let request: CancelVoiceRoomInvite = decode_payload(&envelope)?;
            match application::cancel_voice_room_invite(state, user_id, request).await {
                Ok(event) => {
                    write_envelope(
                        send,
                        RealtimeModule::VoiceChat,
                        RealtimeKind::VoiceChat(VoiceChatKind::VoiceRoomInviteLifecycleEvent),
                        Some(request_id),
                        event,
                    )
                    .await
                }
                Err(error) => reject_application_error(send, Some(request_id), error).await,
            }
        }
        RealtimeKind::VoiceChat(VoiceChatKind::ListVoiceRoomInvites) => {
            let request_id = require_request_id(&envelope)?;
            let request: ListVoiceRoomInvites = decode_payload(&envelope)?;
            match application::list_voice_room_invites(state, user_id, request).await {
                Ok(snapshot) => {
                    write_envelope(
                        send,
                        RealtimeModule::VoiceChat,
                        RealtimeKind::VoiceChat(VoiceChatKind::VoiceRoomInvitesSnapshot),
                        Some(request_id),
                        snapshot,
                    )
                    .await
                }
                Err(error) => reject_application_error(send, Some(request_id), error).await,
            }
        }
        _ => unreachable!("в обработчик приглашений передана посторонняя команда"),
    }
}
