//! Network realtime helpers.

use cheenhub_contracts::realtime::{NetworkKind, Ping, Pong, RealtimeKind, RealtimeModule};
use dioxus::prelude::warn;
use web_time::Instant;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Completed network ping-pong measurement.
pub(crate) struct PingMeasurement {
    /// Client timestamp when the pong arrived.
    pub(crate) received_at_ms: u64,
    /// Client-measured round-trip time in milliseconds.
    pub(crate) rtt_ms: f64,
}

/// Sends one reliable network ping and waits for the pong response.
pub(crate) async fn ping(realtime: &RealtimeHandle) -> Result<PingMeasurement, RealtimeError> {
    let sent_at_ms = now_ms();
    let sent_at = Instant::now();
    let _: Pong = realtime
        .request(
            RealtimeModule::Network,
            RealtimeKind::Network(NetworkKind::Ping),
            Ping { sent_at_ms },
        )
        .await?;
    let received_at_ms = now_ms();
    let rtt_ms = sent_at.elapsed().as_secs_f64() * 1_000.0;

    Ok(PingMeasurement {
        received_at_ms,
        rtt_ms,
    })
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
