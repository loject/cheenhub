//! Control realtime module.

use cheenhub_contracts::realtime::{
    Authenticate, Authenticated, ControlAck, ControlKind, ControlText, RealtimeEnvelope,
    RealtimeKind, RealtimeModule, RejectionCode,
};
use tokio::sync::Mutex;
use tracing::warn;
use web_transport::SendStream;

use crate::features::auth::application as auth_application;
use crate::state::AppState;

use super::protocol::{
    decode_payload, require_request_id, send_rejection, validate_envelope, write_envelope,
};

/// Authenticates the first stream of a realtime session.
pub(crate) async fn authenticate_session(
    state: &AppState,
    send: &Mutex<SendStream>,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<bool> {
    validate_envelope(&envelope)?;

    if envelope.module != RealtimeModule::Control
        || envelope.kind != RealtimeKind::Control(ControlKind::Authenticate)
    {
        send_rejection(
            send,
            envelope.request_id,
            RejectionCode::Unauthorized,
            "Первое realtime сообщение должно авторизовать сессию.",
        )
        .await?;
        return Ok(false);
    }

    let request_id = require_request_id(&envelope)?;
    let auth: Authenticate = decode_payload(&envelope)?;
    let user = match auth_application::me(state, &auth.access_token).await {
        Ok(user) => user,
        Err(error) => {
            warn!(?error, "rejected realtime authentication");
            send_rejection(
                send,
                Some(request_id),
                RejectionCode::Unauthorized,
                "Сессия истекла. Войди снова.",
            )
            .await?;
            return Ok(false);
        }
    };

    write_envelope(
        send,
        RealtimeModule::Control,
        RealtimeKind::Control(ControlKind::Authenticated),
        Some(request_id),
        Authenticated { user },
    )
    .await?;

    Ok(true)
}

/// Handles one control module envelope.
pub(crate) async fn handle(
    _state: &AppState,
    send: &Mutex<SendStream>,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<()> {
    match envelope.kind {
        RealtimeKind::Control(ControlKind::ControlText) => {
            let request_id = require_request_id(&envelope)?;
            let payload: ControlText = decode_payload(&envelope)?;
            write_envelope(
                send,
                RealtimeModule::Control,
                RealtimeKind::Control(ControlKind::ControlAck),
                Some(request_id),
                ControlAck {
                    body: format!("received: {}", payload.body),
                },
            )
            .await
        }
        RealtimeKind::Control(ControlKind::Authenticate) => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::BadRequest,
                "Realtime session is already authenticated.",
            )
            .await
        }
        RealtimeKind::Control(_) => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::UnsupportedMessage,
                "Unsupported control message.",
            )
            .await
        }
        _ => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::BadRequest,
                "Realtime kind does not belong to control module.",
            )
            .await
        }
    }
}
