//! Исходящие приемники realtime для поддерживаемых транспортов.

use std::sync::Arc;

use anyhow::{Context, anyhow};
use axum::extract::ws::Utf8Bytes;
use bytes::Bytes;
use cheenhub_contracts::realtime::{RealtimeEnvelope, RealtimeKind, RealtimeModule};
use serde::Serialize;
use tokio::sync::{Mutex, mpsc};
use web_transport::{SendStream, Session};

use super::framing;

/// Исходящее сообщение, записываемое адаптером realtime WebSocket.
pub(crate) enum WebSocketOutbound {
    /// Надежный realtime-конверт, закодированный как текстовое сообщение WebSocket.
    Envelope(RealtimeEnvelope),
    /// Один раз сериализованный текстовый конверт для fanout.
    PreparedEnvelope(Utf8Bytes),
    /// Байты медиадатаграммы, закодированные как двоичное сообщение WebSocket.
    Datagram(Bytes),
    /// Close-кадр с кодом причины, например при плановом рестарте backend.
    Close {
        /// Код закрытия по RFC 6455.
        code: u32,
        /// Человекочитаемая причина закрытия.
        reason: &'static str,
    },
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

/// Общий неизменяемый пакет для надежной рассылки по двум транспортам.
pub(crate) struct PreparedEnvelope {
    websocket_text: Utf8Bytes,
    webtransport_frame: Bytes,
}

impl PreparedEnvelope {
    /// Сериализует конверт и готовит оба транспортных представления один раз.
    pub(crate) fn new<T: Serialize>(
        module: RealtimeModule,
        kind: RealtimeKind,
        payload: T,
    ) -> anyhow::Result<Self> {
        let envelope = RealtimeEnvelope::new(module, kind, None, payload)
            .context("failed to build realtime fanout envelope")?;
        let json =
            serde_json::to_vec(&envelope).context("failed to encode realtime fanout envelope")?;
        let webtransport_frame = framing::prepare_frame(&json)?;
        let websocket_text = Utf8Bytes::try_from(Bytes::from(json))
            .context("serialized realtime envelope is not UTF-8")?;
        Ok(Self {
            websocket_text,
            webtransport_frame,
        })
    }
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

    /// Рассылает подготовленный конверт без повторной сериализации или копии тела.
    pub(crate) async fn send_prepared_envelope(
        &self,
        prepared: &PreparedEnvelope,
    ) -> anyhow::Result<()> {
        match self {
            Self::WebTransport(send) => {
                framing::write_prepared_frame(send, &prepared.webtransport_frame).await
            }
            Self::WebSocket(sender) => sender
                .try_send(WebSocketOutbound::PreparedEnvelope(
                    prepared.websocket_text.clone(),
                ))
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
mod tests;
