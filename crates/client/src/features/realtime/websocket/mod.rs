//! WebSocket fallback realtime-транспорта.

mod native;
mod unsupported;
mod web;

use bytes::Bytes;
use cheenhub_contracts::realtime::RealtimeEnvelope;
use dioxus::prelude::{debug, warn};
use futures_channel::mpsc;
use futures_util::future::{AbortHandle, AbortRegistration};

use super::handle::DatagramListeners;

pub(super) use native::{spawn_reader, spawn_writer, split};

/// Отправка сообщений и независимая отмена обеих половин WebSocket.
#[derive(Clone)]
pub(super) struct WebSocketOutboundSender {
    sender: mpsc::UnboundedSender<WebSocketOutbound>,
    writer_abort: AbortHandle,
    reader_abort: AbortHandle,
}

impl WebSocketOutboundSender {
    /// Создаёт отправителя и отдельные регистрации отмены writer/reader.
    pub(super) fn new(
        sender: mpsc::UnboundedSender<WebSocketOutbound>,
    ) -> (Self, AbortRegistration, AbortRegistration) {
        let (writer_abort, writer_registration) = AbortHandle::new_pair();
        let (reader_abort, reader_registration) = AbortHandle::new_pair();
        (
            Self {
                sender,
                writer_abort,
                reader_abort,
            },
            writer_registration,
            reader_registration,
        )
    }

    /// Отправляет сообщение текущему writer.
    pub(super) fn unbounded_send(
        &self,
        message: WebSocketOutbound,
    ) -> Result<(), mpsc::TrySendError<WebSocketOutbound>> {
        self.sender.unbounded_send(message)
    }

    /// Прекращает очередь и отменяет обе задачи, включая зависшие операции I/O.
    pub(super) fn close(&self) {
        self.sender.close_channel();
        self.writer_abort.abort();
        self.reader_abort.abort();
    }
}

/// Исходящее сообщение WebSocket fallback.
pub(super) enum WebSocketOutbound {
    /// Realtime-конверт поверх надежного WebSocket-сообщения.
    Envelope(RealtimeEnvelope),
    /// Датаграмма, отправленная через WebSocket при недоступности WebTransport.
    Datagram(Bytes),
}

fn dispatch_text_envelope(
    url: &str,
    generation: u64,
    text: &str,
    inbound: &mpsc::UnboundedSender<RealtimeEnvelope>,
) -> bool {
    let envelope = match serde_json::from_str::<RealtimeEnvelope>(text) {
        Ok(envelope) => envelope,
        Err(error) => {
            warn!(
                %url,
                %generation,
                %error,
                "failed to decode WebSocket realtime envelope"
            );
            return false;
        }
    };
    if !envelope.has_matching_module_kind() {
        warn!(
            %url,
            %generation,
            envelope_module = ?envelope.module,
            envelope_kind = ?envelope.kind,
            "closing WebSocket fallback after mismatched envelope"
        );
        return false;
    }
    if inbound.unbounded_send(envelope).is_err() {
        debug!(%url, %generation, "realtime inbound dispatcher closed");
        return false;
    }

    true
}

fn dispatch_datagram(bytes: Bytes, datagram_listeners: &DatagramListeners) {
    datagram_listeners
        .borrow_mut()
        .retain(|listener| listener.unbounded_send(bytes.clone()).is_ok());
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{FutureExt, future::Abortable};
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn shutdown_interrupts_blocked_writer_and_reader_without_draining_queue() {
        struct Dropped(Rc<Cell<bool>>);
        impl Drop for Dropped {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }
        let (sender, _receiver) = mpsc::unbounded();
        let cloned_sender = sender.clone();
        let (sender, writer_cancel, reader_cancel) = WebSocketOutboundSender::new(sender);
        let writer_dropped = Rc::new(Cell::new(false));
        let reader_dropped = Rc::new(Cell::new(false));
        let writer_guard = Dropped(writer_dropped.clone());
        let reader_guard = Dropped(reader_dropped.clone());
        let mut writer = Box::pin(async {
            Abortable::new(
                async move {
                    let _guard = writer_guard;
                    std::future::pending::<()>().await;
                },
                writer_cancel,
            )
            .await
        });
        let mut reader = Box::pin(async {
            Abortable::new(
                async move {
                    let _guard = reader_guard;
                    std::future::pending::<()>().await;
                },
                reader_cancel,
            )
            .await
        });
        assert!(writer.as_mut().now_or_never().is_none());
        assert!(reader.as_mut().now_or_never().is_none());
        sender
            .unbounded_send(WebSocketOutbound::Datagram(bytes::Bytes::from_static(
                b"queued",
            )))
            .unwrap();
        sender.close();
        assert!(matches!(writer.as_mut().now_or_never(), Some(Err(_))));
        assert!(matches!(reader.as_mut().now_or_never(), Some(Err(_))));
        assert!(writer_dropped.get());
        assert!(reader_dropped.get());
        assert!(cloned_sender.is_closed());
    }
}
