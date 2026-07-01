//! Fire-and-forget realtime send helpers.

use bytes::Bytes;
use cheenhub_contracts::realtime::{RealtimeEnvelope, RealtimeKind, RealtimeModule};
use serde::Serialize;

use crate::features::realtime::error::RealtimeError;
use crate::features::realtime::websocket::WebSocketOutbound;

use super::{ConnectedTransport, RealtimeHandle, ReliableRequestMode, validate_module_kind};

impl RealtimeHandle {
    /// Sends one reliable fire-and-forget message.
    #[allow(dead_code)]
    pub(crate) async fn send_reliable<P>(
        &self,
        module: RealtimeModule,
        kind: RealtimeKind,
        payload: P,
    ) -> Result<(), RealtimeError>
    where
        P: Serialize,
    {
        validate_module_kind(module, kind)?;
        let envelope = RealtimeEnvelope::new(module, kind, None, payload).map_err(|error| {
            RealtimeError::new(format!("Failed to encode realtime payload: {error}"))
        })?;
        self.write_envelope(envelope, ReliableRequestMode::Cached)
            .await
    }

    /// Sends one unreliable datagram message.
    #[allow(dead_code)]
    pub(crate) async fn send_unreliable<P>(
        &self,
        module: RealtimeModule,
        kind: RealtimeKind,
        payload: P,
    ) -> Result<(), RealtimeError>
    where
        P: Serialize,
    {
        validate_module_kind(module, kind)?;
        let envelope = RealtimeEnvelope::new(module, kind, None, payload).map_err(|error| {
            RealtimeError::new(format!("Failed to encode realtime payload: {error}"))
        })?;
        let Some(connected) = self.inner.session.lock().await.clone() else {
            return Err(RealtimeError::new("Realtime session is not connected."));
        };

        match connected.transport {
            ConnectedTransport::WebTransport(session) => {
                let bytes = serde_json::to_vec(&envelope).map_err(|error| {
                    RealtimeError::new(format!("Failed to encode realtime datagram: {error}"))
                })?;
                let _write_guard = self.inner.datagram_writes.lock().await;
                session
                    .send_datagram(Bytes::from(bytes))
                    .await
                    .map_err(|error| {
                        RealtimeError::new(format!("Failed to send realtime datagram: {error}"))
                    })
            }
            ConnectedTransport::WebSocket(sender) => sender
                .unbounded_send(WebSocketOutbound::Envelope(envelope))
                .map_err(|_| RealtimeError::new("Realtime WebSocket fallback writer is closed.")),
        }
    }
}
