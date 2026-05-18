//! Realtime module router.

use cheenhub_contracts::realtime::{RealtimeEnvelope, RealtimeModule, RejectionCode};
use cheenhub_contracts::rest::AuthUser;
use uuid::Uuid;

use crate::features::{servers, text_chat, voice_chat};
use crate::state::AppState;

use super::protocol::send_rejection;
use super::sink::EnvelopeSink;
use super::{control, network};

/// Dispatches a realtime envelope to the module that owns it.
pub(crate) async fn dispatch(
    state: &AppState,
    user: &AuthUser,
    user_id: &Uuid,
    stream_id: Uuid,
    session_id: Uuid,
    send: &EnvelopeSink,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<()> {
    match envelope.module {
        RealtimeModule::Control => control::handle(state, send, envelope).await,
        RealtimeModule::Network => network::handle(state, send, envelope).await,
        RealtimeModule::Server => servers::realtime::handle(state, user_id, send, envelope).await,
        RealtimeModule::TextChat => {
            text_chat::realtime::handle(state, user, user_id, send, envelope).await
        }
        RealtimeModule::VoiceChat => {
            voice_chat::realtime::handle(
                state, user, user_id, stream_id, session_id, send, envelope,
            )
            .await
        }
    }
}

/// Runs module-specific cleanup after a reliable stream closes.
pub(crate) async fn cleanup_stream(state: &AppState, module: RealtimeModule, stream_id: Uuid) {
    match module {
        RealtimeModule::VoiceChat => {
            voice_chat::application::disconnect_realtime_stream(state, stream_id).await;
        }
        RealtimeModule::Control
        | RealtimeModule::Network
        | RealtimeModule::Server
        | RealtimeModule::TextChat => {}
    }
}

/// Rejects a reliable stream message when it changes module ownership.
pub(crate) async fn reject_module_change(
    send: &EnvelopeSink,
    envelope: &RealtimeEnvelope,
) -> anyhow::Result<()> {
    send_rejection(
        send,
        envelope.request_id,
        RejectionCode::BadRequest,
        "Reliable stream already belongs to another realtime module.",
    )
    .await
}
