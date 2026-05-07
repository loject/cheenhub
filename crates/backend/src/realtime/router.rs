//! Realtime module router.

use cheenhub_contracts::realtime::{RealtimeEnvelope, RealtimeModule, RejectionCode};
use cheenhub_contracts::rest::AuthUser;
use tokio::sync::Mutex;
use uuid::Uuid;
use web_transport::SendStream;

use crate::features::text_chat;
use crate::state::AppState;

use super::protocol::send_rejection;
use super::{control, network};

/// Dispatches a realtime envelope to the module that owns it.
pub(crate) async fn dispatch(
    state: &AppState,
    user: &AuthUser,
    user_id: &Uuid,
    send: &Mutex<SendStream>,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<()> {
    match envelope.module {
        RealtimeModule::Control => control::handle(state, send, envelope).await,
        RealtimeModule::Network => network::handle(state, send, envelope).await,
        RealtimeModule::TextChat => {
            text_chat::realtime::handle(state, user, user_id, send, envelope).await
        }
    }
}

/// Rejects a reliable stream message when it changes module ownership.
pub(crate) async fn reject_module_change(
    send: &Mutex<SendStream>,
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
