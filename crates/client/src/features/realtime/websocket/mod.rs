//! WebSocket fallback realtime-транспорта.

mod native;
mod unsupported;
mod web;

use bytes::Bytes;
use cheenhub_contracts::realtime::RealtimeEnvelope;
use dioxus::prelude::{debug, warn};
use futures_channel::mpsc;

use super::handle::DatagramListeners;

pub(super) use native::{spawn_reader, spawn_writer, split};

pub(super) type WebSocketOutboundSender = mpsc::UnboundedSender<WebSocketOutbound>;

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
