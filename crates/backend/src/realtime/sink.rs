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
    WebSocket(mpsc::Sender<WebSocketOutbound>),
}

/// Конкретный отправитель датаграмм для медиа-сообщений realtime.
#[derive(Clone)]
pub(crate) enum DatagramSink {
    /// Датаграммы сессии WebTransport.
    WebTransport(Arc<Session>),
    /// Двоичный писатель WebSocket-резерва.
    WebSocket(mpsc::Sender<WebSocketOutbound>),
}

impl EnvelopeSink {
    /// Оборачивает надежный поток WebTransport.
    pub(crate) fn webtransport(send: Arc<Mutex<SendStream>>) -> Self {
        Self::WebTransport(send)
    }

    /// Оборачивает писатель WebSocket-резерва.
    pub(crate) fn websocket(sender: mpsc::Sender<WebSocketOutbound>) -> Self {
        Self::WebSocket(sender)
    }

    /// Отправляет один надежный realtime-конверт.
    pub(crate) async fn send_envelope(&self, envelope: &RealtimeEnvelope) -> anyhow::Result<()> {
        match self {
            Self::WebTransport(send) => framing::write_envelope(send, envelope).await,
            Self::WebSocket(sender) => sender
                .try_send(WebSocketOutbound::Envelope(envelope.clone()))
                .map_err(|error| match error {
                    mpsc::error::TrySendError::Full(_) => {
                        anyhow!("websocket realtime outbound queue is full")
                    }
                    mpsc::error::TrySendError::Closed(_) => {
                        anyhow!("websocket realtime writer is closed")
                    }
                }),
        }
    }
}

impl DatagramSink {
    /// Оборачивает сессию WebTransport.
    pub(crate) fn webtransport(session: Session) -> Self {
        Self::WebTransport(Arc::new(session))
    }

    /// Оборачивает писатель WebSocket-резерва.
    pub(crate) fn websocket(sender: mpsc::Sender<WebSocketOutbound>) -> Self {
        Self::WebSocket(sender)
    }

    /// Отправляет одну медиадатаграмму через активный транспорт.
    pub(crate) async fn send_datagram(&self, bytes: Bytes) -> anyhow::Result<()> {
        match self {
            Self::WebTransport(session) => session
                .send_datagram(bytes)
                .await
                .context("failed to send WebTransport datagram"),
            // Медиадатаграммы не задерживают общий fanout из-за медленного WebSocket-клиента.
            Self::WebSocket(sender) => match sender.try_send(WebSocketOutbound::Datagram(bytes)) {
                Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => Ok(()),
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    Err(anyhow!("websocket realtime writer is closed"))
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use cheenhub_contracts::realtime::{NetworkKind, Ping, RealtimeKind, RealtimeModule};

    use super::*;

    #[tokio::test]
    async fn websocket_envelope_reports_full_outbound_queue() {
        let (sender, _receiver) = mpsc::channel(1);
        sender
            .try_send(WebSocketOutbound::Datagram(Bytes::from_static(b"occupied")))
            .expect("очередь принимает первое сообщение");
        let sink = EnvelopeSink::websocket(sender);
        let envelope = RealtimeEnvelope::new(
            RealtimeModule::Network,
            RealtimeKind::Network(NetworkKind::Ping),
            None,
            Ping { sent_at_ms: 1 },
        )
        .expect("конверт сериализуется");

        let error = sink
            .send_envelope(&envelope)
            .await
            .expect_err("переполненная очередь отклоняет надёжное сообщение");

        assert!(error.to_string().contains("outbound queue is full"));
    }

    #[tokio::test]
    async fn websocket_datagram_is_dropped_when_outbound_queue_is_full() {
        let (sender, mut receiver) = mpsc::channel(1);
        sender
            .try_send(WebSocketOutbound::Datagram(Bytes::from_static(b"first")))
            .expect("очередь принимает первую датаграмму");
        let sink = DatagramSink::websocket(sender);

        sink.send_datagram(Bytes::from_static(b"second"))
            .await
            .expect("переполнение не замедляет fanout");

        let Some(WebSocketOutbound::Datagram(bytes)) = receiver.recv().await else {
            panic!("в очереди должна остаться первая датаграмма");
        };
        assert_eq!(bytes, Bytes::from_static(b"first"));
        assert!(receiver.try_recv().is_err());
    }
}
