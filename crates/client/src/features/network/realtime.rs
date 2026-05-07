//! Network realtime helpers.

use cheenhub_contracts::realtime::{NetworkKind, Ping, Pong, RealtimeKind, RealtimeModule};
use dioxus::prelude::{debug, warn};

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Sends one reliable network ping and waits for the pong response.
pub(crate) async fn ping(realtime: &RealtimeHandle) -> Result<Pong, RealtimeError> {
    let sent_at_ms = now_ms();
    debug!("rt ping");
    let pong: Pong = realtime
        .request(
            RealtimeModule::Network,
            RealtimeKind::Network(NetworkKind::Ping),
            Ping { sent_at_ms },
        )
        .await?;
    let received_at = now_ms();
    debug!(
        rtt_ms = received_at.saturating_sub(pong.sent_at_ms),
        srv_ms = pong
            .server_sent_at_ms
            .saturating_sub(pong.server_received_at_ms),
        "rt pong"
    );

    Ok(pong)
}

fn now_ms() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or_else(|error| {
            warn!(%error, "failed to convert network timestamp");
            u64::MAX
        })
}
