//! Native-реализация WebSocket fallback через tokio-tungstenite.

use dioxus::prelude::{debug, info, warn};
use futures_channel::mpsc;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

use super::super::{WebSocketOutbound, dispatch_datagram, dispatch_text_envelope};
use crate::features::realtime::error::RealtimeError;
use crate::features::realtime::handle::{DatagramListeners, RealtimeHandle};
use crate::features::realtime::task::spawn_task;

type NativeWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub(in crate::features::realtime) type WebSocketWriter =
    futures_util::stream::SplitSink<NativeWebSocket, Message>;
pub(in crate::features::realtime) type WebSocketReader =
    futures_util::stream::SplitStream<NativeWebSocket>;

pub(in crate::features::realtime) async fn split(
    url: &str,
) -> Result<(WebSocketWriter, WebSocketReader), RealtimeError> {
    let (websocket, _) = connect_async(url).await.map_err(|error| {
        RealtimeError::new(format!(
            "Failed to open WebSocket realtime fallback: {error}"
        ))
    })?;

    Ok(websocket.split())
}

pub(in crate::features::realtime) fn spawn_writer(
    url: String,
    generation: u64,
    mut writer: WebSocketWriter,
    mut outbound: mpsc::UnboundedReceiver<WebSocketOutbound>,
    realtime: Option<RealtimeHandle>,
) {
    spawn_task(async move {
        while let Some(message) = outbound.next().await {
            let message = match encode_message(message, &url, generation) {
                Some(message) => message,
                None => continue,
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

fn encode_message(message: WebSocketOutbound, url: &str, generation: u64) -> Option<Message> {
    match message {
        WebSocketOutbound::Envelope(envelope) => match serde_json::to_string(&envelope) {
            Ok(json) => Some(Message::text(json)),
            Err(error) => {
                warn!(
                    %url,
                    %generation,
                    %error,
                    "failed to encode WebSocket realtime envelope"
                );
                None
            }
        },
        WebSocketOutbound::Datagram(bytes) => Some(Message::binary(bytes)),
    }
}

pub(in crate::features::realtime) fn spawn_reader(
    url: String,
    generation: u64,
    mut reader: WebSocketReader,
    inbound: mpsc::UnboundedSender<cheenhub_contracts::realtime::RealtimeEnvelope>,
    datagram_listeners: DatagramListeners,
    realtime: RealtimeHandle,
) {
    spawn_task(async move {
        while let Some(message) = reader.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if !dispatch_text_envelope(&url, generation, text.as_str(), &inbound) {
                        break;
                    }
                }
                Ok(Message::Binary(bytes)) => {
                    dispatch_datagram(bytes, &datagram_listeners);
                }
                Ok(Message::Close(_)) => {
                    debug!(%url, %generation, "WebSocket realtime fallback closed by peer");
                    break;
                }
                Ok(_) => {}
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
