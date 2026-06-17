//! Исходящие приемники realtime для поддерживаемых транспортов.

use std::sync::Arc;

use anyhow::{Context, anyhow};
use bytes::Bytes;
use cheenhub_contracts::realtime::RealtimeEnvelope;
use tokio::sync::{Mutex, mpsc};
use web_transport::{SendStream, Session};

use super::framing;

/// Исходящее сообщение, записываемое адаптером realtime WebSocket.
pub(crate) enum WebSocketOutbound {
    /// Надежный realtime-конверт, закодированный как текстовое сообщение WebSocket.
    Envelope(RealtimeEnvelope),
    /// Байты медиадатаграммы, закодированные как двоичное сообщение WebSocket.
    Datagram(Bytes),
}

/// Конкретный отправитель конвертов для надежных realtime-сообщений.
#[derive(Clone)]
pub(crate) enum EnvelopeSink {
    /// Двунаправленный надежный поток WebTransport.
    WebTransport(Arc<Mutex<SendStream>>),
    /// Запись соединения WebSocket-резерва.
    WebSocket(mpsc::UnboundedSender<WebSocketOutbound>),
}

/// Конкретный отправитель датаграмм для медиа-сообщений realtime.
#[derive(Clone)]
pub(crate) enum DatagramSink {
    /// Датаграммы сессии WebTransport.
    WebTransport(Arc<Session>),
    /// Двоичный писатель WebSocket-резерва.
    WebSocket(mpsc::UnboundedSender<WebSocketOutbound>),
}

impl EnvelopeSink {
    /// Оборачивает надежный поток WebTransport.
    pub(crate) fn webtransport(send: Arc<Mutex<SendStream>>) -> Self {
        Self::WebTransport(send)
    }

    /// Оборачивает писатель WebSocket-резерва.
    pub(crate) fn websocket(sender: mpsc::UnboundedSender<WebSocketOutbound>) -> Self {
        Self::WebSocket(sender)
    }

    /// Отправляет один надежный realtime-конверт.
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
    /// Оборачивает сессию WebTransport.
    pub(crate) fn webtransport(session: Session) -> Self {
        Self::WebTransport(Arc::new(session))
    }

    /// Оборачивает писатель WebSocket-резерва.
    pub(crate) fn websocket(sender: mpsc::UnboundedSender<WebSocketOutbound>) -> Self {
        Self::WebSocket(sender)
    }

    /// Отправляет одну медиадатаграмму через активный транспорт.
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
