//! Модуль оценки качества сети realtime.

use std::time::{SystemTime, UNIX_EPOCH};

use cheenhub_contracts::realtime::{
    NetworkKind, Ping, Pong, RealtimeEnvelope, RealtimeKind, RealtimeModule, RejectionCode,
};

use crate::state::AppState;

use super::protocol::{decode_payload, require_request_id, send_rejection, write_envelope};
use super::sink::EnvelopeSink;

/// Обрабатывает один конверт сетевого модуля.
pub(crate) async fn handle(
    _state: &AppState,
    send: &EnvelopeSink,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<()> {
    match envelope.kind {
        RealtimeKind::Network(NetworkKind::Ping) => {
            let request_id = require_request_id(&envelope)?;
            let received_at = now_ms();
            let payload: Ping = decode_payload(&envelope)?;
            let server_sent_at = now_ms();
            write_envelope(
                send,
                RealtimeModule::Network,
                RealtimeKind::Network(NetworkKind::Pong),
                Some(request_id),
                Pong {
                    sent_at_ms: payload.sent_at_ms,
                    server_received_at_ms: received_at,
                    server_sent_at_ms: server_sent_at,
                },
            )
            .await
        }
        RealtimeKind::Network(_) => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::UnsupportedMessage,
                "Unsupported network message.",
            )
            .await
        }
        _ => {
            send_rejection(
                send,
                envelope.request_id,
                RejectionCode::BadRequest,
                "Realtime kind does not belong to network module.",
            )
            .await
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
