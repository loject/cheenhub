//! Заглушка WebSocket fallback для несовместимой комбинации target/feature.
#![cfg_attr(
    not(all(
        not(target_arch = "wasm32"),
        not(any(feature = "desktop", feature = "mobile"))
    )),
    allow(dead_code, unused_imports)
)]

use cheenhub_contracts::realtime::RealtimeEnvelope;
use dioxus::prelude::warn;
use futures_channel::mpsc;

use super::WebSocketOutbound;
use crate::features::realtime::error::RealtimeError;
use crate::features::realtime::handle::{DatagramListeners, RealtimeHandle};

pub(in crate::features::realtime) struct WebSocketWriter;

pub(in crate::features::realtime) struct WebSocketReader;

pub(in crate::features::realtime) async fn split(
    url: &str,
) -> Result<(WebSocketWriter, WebSocketReader), RealtimeError> {
    warn!(%url, "skipping WebSocket realtime fallback for unsupported client target");
    Err(RealtimeError::new(
        "WebSocket realtime fallback is not available for this client target.",
    ))
}

pub(in crate::features::realtime) fn spawn_writer(
    _url: String,
    _generation: u64,
    _writer: WebSocketWriter,
    _outbound: mpsc::UnboundedReceiver<WebSocketOutbound>,
    _realtime: Option<RealtimeHandle>,
) {
}

pub(in crate::features::realtime) fn spawn_reader(
    _url: String,
    _generation: u64,
    _reader: WebSocketReader,
    _inbound: mpsc::UnboundedSender<RealtimeEnvelope>,
    _datagram_listeners: DatagramListeners,
    _realtime: RealtimeHandle,
) {
}
