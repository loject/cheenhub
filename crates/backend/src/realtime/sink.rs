//! Realtime outbound sinks for supported transports.

use std::sync::Arc;

use anyhow::{Context, anyhow};
use bytes::Bytes;
use cheenhub_contracts::realtime::RealtimeEnvelope;
use tokio::sync::{Mutex, mpsc};
use web_transport::{SendStream, Session};

use super::framing;

/// Outbound message written by the WebSocket realtime adapter.
pub(crate) enum WebSocketOutbound {
    /// Reliable realtime envelope encoded as a WebSocket text message.
    Envelope(RealtimeEnvelope),
    /// Media datagram bytes encoded as a WebSocket binary message.
    Datagram(Bytes),
}

/// Concrete envelope sender for realtime reliable messages.
#[derive(Clone)]
pub(crate) enum EnvelopeSink {
    /// WebTransport bidirectional reliable stream.
    WebTransport(Arc<Mutex<SendStream>>),
    /// WebSocket fallback connection writer.
    WebSocket(mpsc::UnboundedSender<WebSocketOutbound>),
}

/// Concrete datagram sender for realtime media messages.
#[derive(Clone)]
pub(crate) enum DatagramSink {
    /// WebTransport session datagrams.
    WebTransport(Arc<Session>),
    /// WebSocket fallback binary writer.
    WebSocket(mpsc::UnboundedSender<WebSocketOutbound>),
}

impl EnvelopeSink {
    /// Wraps a WebTransport reliable stream.
    pub(crate) fn webtransport(send: Arc<Mutex<SendStream>>) -> Self {
        Self::WebTransport(send)
    }

    /// Wraps a WebSocket fallback writer.
    pub(crate) fn websocket(sender: mpsc::UnboundedSender<WebSocketOutbound>) -> Self {
        Self::WebSocket(sender)
    }

    /// Sends one reliable realtime envelope.
    pub(crate) async fn send_envelope(&self, envelope: &RealtimeEnvelope) -> anyhow::Result<()> {
        match self {
            Self::WebTransport(send) => framing::write_envelope(send, envelope).await,
            Self::WebSocket(sender) => sender
                .send(WebSocketOutbound::Envelope(envelope.clone()))
                .map_err(|_| anyhow!("websocket realtime writer is closed")),
        }
    }
}

impl DatagramSink {
    /// Wraps a WebTransport session.
    pub(crate) fn webtransport(session: Session) -> Self {
        Self::WebTransport(Arc::new(session))
    }

    /// Wraps a WebSocket fallback writer.
    pub(crate) fn websocket(sender: mpsc::UnboundedSender<WebSocketOutbound>) -> Self {
        Self::WebSocket(sender)
    }

    /// Sends one media datagram through the active transport.
    pub(crate) async fn send_datagram(&self, bytes: Bytes) -> anyhow::Result<()> {
        match self {
            Self::WebTransport(session) => session
                .send_datagram(bytes)
                .await
                .context("failed to send WebTransport datagram"),
            Self::WebSocket(sender) => sender
                .send(WebSocketOutbound::Datagram(bytes))
                .map_err(|_| anyhow!("websocket realtime writer is closed")),
        }
    }
}
