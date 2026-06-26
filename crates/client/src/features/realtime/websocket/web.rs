//! Web-реализация WebSocket fallback через browser WebSocket API.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use bytes::Bytes;
use dioxus::prelude::{info, warn};
use futures_channel::{mpsc, oneshot};
use futures_util::StreamExt;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{BinaryType, Event, MessageEvent, WebSocket};

use super::{WebSocketOutbound, dispatch_datagram, dispatch_text_envelope};
use crate::features::realtime::error::RealtimeError;
use crate::features::realtime::handle::{DatagramListeners, RealtimeHandle};
use crate::features::realtime::task::spawn_task;

pub(in crate::features::realtime) struct WebSocketWriter {
    websocket: WebSocket,
}

pub(in crate::features::realtime) struct WebSocketReader {
    websocket: WebSocket,
}

pub(in crate::features::realtime) async fn split(
    url: &str,
) -> Result<(WebSocketWriter, WebSocketReader), RealtimeError> {
    let websocket = WebSocket::new(url).map_err(|error| {
        RealtimeError::new(format!(
            "Failed to open WebSocket realtime fallback: {}",
            js_error_message(error)
        ))
    })?;
    websocket.set_binary_type(BinaryType::Arraybuffer);
    wait_until_open(&websocket).await?;

    Ok((
        WebSocketWriter {
            websocket: websocket.clone(),
        },
        WebSocketReader { websocket },
    ))
}

pub(in crate::features::realtime) fn spawn_writer(
    url: String,
    generation: u64,
    writer: WebSocketWriter,
    mut outbound: mpsc::UnboundedReceiver<WebSocketOutbound>,
    realtime: Option<RealtimeHandle>,
) {
    spawn_task(async move {
        while let Some(message) = outbound.next().await {
            let result = match message {
                WebSocketOutbound::Envelope(envelope) => match serde_json::to_string(&envelope) {
                    Ok(json) => writer.websocket.send_with_str(&json),
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
                WebSocketOutbound::Datagram(bytes) => {
                    writer.websocket.send_with_u8_array(bytes.as_ref())
                }
            };

            if let Err(error) = result {
                warn!(
                    %url,
                    %generation,
                    error = %js_error_message(error),
                    "WebSocket realtime fallback write failed"
                );
                let _ = writer.websocket.close();
                if let Some(realtime) = &realtime {
                    realtime.clear_generation(generation).await;
                }
                break;
            }
        }
    });
}

pub(in crate::features::realtime) fn spawn_reader(
    url: String,
    generation: u64,
    reader: WebSocketReader,
    inbound: mpsc::UnboundedSender<cheenhub_contracts::realtime::RealtimeEnvelope>,
    datagram_listeners: DatagramListeners,
    realtime: RealtimeHandle,
) {
    let message_url = url.clone();
    let message_websocket = reader.websocket.clone();
    let message_closure = Closure::wrap(Box::new(move |event: MessageEvent| {
        let data = event.data();
        if let Some(text) = data.as_string() {
            if !dispatch_text_envelope(&message_url, generation, &text, &inbound) {
                let _ = message_websocket.close();
            }
            return;
        }

        let Some(bytes) = bytes_from_message_data(data) else {
            warn!(
                %message_url,
                %generation,
                "received unsupported WebSocket realtime fallback message"
            );
            let _ = message_websocket.close();
            return;
        };
        dispatch_datagram(bytes, &datagram_listeners);
    }) as Box<dyn FnMut(MessageEvent)>);

    let error_url = url.clone();
    let error_closure = Closure::wrap(Box::new(move |_event: Event| {
        warn!(
            %error_url,
            %generation,
            "WebSocket realtime fallback browser error"
        );
    }) as Box<dyn FnMut(Event)>);

    let (close_sender, close_receiver) = oneshot::channel();
    let close_sender = Rc::new(RefCell::new(Some(close_sender)));
    let close_state = close_sender.clone();
    let close_closure = Closure::wrap(Box::new(move |_event: Event| {
        if let Some(sender) = close_state.borrow_mut().take() {
            let _ = sender.send(());
        }
    }) as Box<dyn FnMut(Event)>);

    reader
        .websocket
        .set_onmessage(Some(message_closure.as_ref().unchecked_ref()));
    reader
        .websocket
        .set_onerror(Some(error_closure.as_ref().unchecked_ref()));
    reader
        .websocket
        .set_onclose(Some(close_closure.as_ref().unchecked_ref()));

    spawn_task(async move {
        let _message_closure = message_closure;
        let _error_closure = error_closure;
        let _close_closure = close_closure;
        let _websocket = reader.websocket;
        let _ = close_receiver.await;
        info!(%url, %generation, "WebSocket realtime fallback session closed");
        realtime.clear_generation(generation).await;
    });
}

async fn wait_until_open(websocket: &WebSocket) -> Result<(), RealtimeError> {
    let (sender, receiver) = oneshot::channel::<Result<(), String>>();
    let sender = Rc::new(RefCell::new(Some(sender)));

    let open_sender = sender.clone();
    let open_closure = Closure::wrap(Box::new(move |_event: Event| {
        if let Some(sender) = open_sender.borrow_mut().take() {
            let _ = sender.send(Ok(()));
        }
    }) as Box<dyn FnMut(Event)>);

    let error_sender = sender.clone();
    let error_closure = Closure::wrap(Box::new(move |_event: Event| {
        if let Some(sender) = error_sender.borrow_mut().take() {
            let _ = sender.send(Err(
                "Browser WebSocket realtime fallback failed to open.".to_owned()
            ));
        }
    }) as Box<dyn FnMut(Event)>);

    websocket.set_onopen(Some(open_closure.as_ref().unchecked_ref()));
    websocket.set_onerror(Some(error_closure.as_ref().unchecked_ref()));

    let opened = receiver
        .await
        .map_err(|_| RealtimeError::new("Browser WebSocket open callback was dropped."))?;

    websocket.set_onopen(None);
    websocket.set_onerror(None);
    drop(open_closure);
    drop(error_closure);

    opened.map_err(RealtimeError::new)
}

fn bytes_from_message_data(data: JsValue) -> Option<Bytes> {
    if !data.is_instance_of::<js_sys::ArrayBuffer>() {
        return None;
    }
    let array = js_sys::Uint8Array::new(&data);
    let mut bytes = vec![0; array.length() as usize];
    array.copy_to(bytes.as_mut_slice());
    Some(Bytes::from(bytes))
}

fn js_error_message(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}
