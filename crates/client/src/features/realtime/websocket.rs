//! WebSocket fallback client transport helpers.

use bytes::Bytes;
use cheenhub_contracts::realtime::RealtimeEnvelope;
use dioxus::prelude::{debug, info, warn};
use futures_channel::mpsc;
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use gloo_net::websocket::{Message, futures::WebSocket};

use super::error::RealtimeError;
use super::handle::RealtimeHandle;
use super::task::spawn_task;

pub(super) type WebSocketOutboundSender = mpsc::UnboundedSender<WebSocketOutbound>;
pub(super) type WebSocketWriter = SplitSink<WebSocket, Message>;
pub(super) type WebSocketReader = SplitStream<WebSocket>;

pub(super) enum WebSocketOutbound {
    Envelope(RealtimeEnvelope),
    Datagram(Bytes),
}

pub(super) fn split(url: &str) -> Result<(WebSocketWriter, WebSocketReader), RealtimeError> {
    let websocket = WebSocket::open(url).map_err(|error| {
        RealtimeError::new(format!(
            "Failed to open WebSocket realtime fallback: {error}"
        ))
    })?;

    Ok(websocket.split())
}

pub(super) fn spawn_writer(
    url: String,
    generation: u64,
    mut writer: WebSocketWriter,
    mut outbound: mpsc::UnboundedReceiver<WebSocketOutbound>,
    realtime: Option<RealtimeHandle>,
) {
    spawn_task(async move {
        while let Some(message) = outbound.next().await {
            let message = match message {
                WebSocketOutbound::Envelope(envelope) => match serde_json::to_string(&envelope) {
                    Ok(json) => Message::Text(json),
                    Err(error) => {
                        warn!(
                            %url,
                            %generation,
                            %error,
                            "failed to encode WebSocket realtime envelope"
                        );
                        continue;
                    }
                },
                WebSocketOutbound::Datagram(bytes) => Message::Bytes(bytes.to_vec()),
            };

            if let Err(error) = writer.send(message).await {
                warn!(
                    %url,
                    %generation,
                    %error,
                    "WebSocket realtime fallback write failed"
                );
                if let Some(realtime) = &realtime {
                    realtime.clear_generation(generation).await;
                }
                break;
            }
        }
    });
}

pub(super) fn spawn_reader(
    url: String,
    generation: u64,
    mut reader: WebSocketReader,
    inbound: mpsc::UnboundedSender<RealtimeEnvelope>,
    datagram_listeners: super::handle::DatagramListeners,
    realtime: RealtimeHandle,
) {
    spawn_task(async move {
        while let Some(message) = reader.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    let envelope = match serde_json::from_str::<RealtimeEnvelope>(&text) {
                        Ok(envelope) => envelope,
                        Err(error) => {
                            warn!(
                                %url,
                                %generation,
                                %error,
                                "failed to decode WebSocket realtime envelope"
                            );
                            break;
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
                        break;
                    }
                    if inbound.unbounded_send(envelope).is_err() {
                        debug!(%url, %generation, "realtime inbound dispatcher closed");
                        break;
                    }
                }
                Ok(Message::Bytes(bytes)) => {
                    let bytes = Bytes::from(bytes);
                    datagram_listeners
                        .borrow_mut()
                        .retain(|listener| listener.unbounded_send(bytes.clone()).is_ok());
                }
                Err(error) => {
                    warn!(
                        %url,
                        %generation,
                        %error,
                        "WebSocket realtime fallback read failed"
                    );
                    break;
                }
            }
        }

        info!(%url, %generation, "WebSocket realtime fallback session closed");
        realtime.clear_generation(generation).await;
    });
}
