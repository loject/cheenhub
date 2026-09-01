//! Browser WebSocket для realtime-журнала бэкенда.

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use std::{cell::RefCell, rc::Rc};

use cheenhub_contracts::rest::HostLogStreamMessage;
use futures_channel::{mpsc, oneshot};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{Event, MessageEvent, WebSocket};

/// Снимает JS callbacks до уничтожения wasm-bindgen `Closure`.
///
/// При cancellation future во время размонтирования Dioxus-компонента
/// браузер не должен сохранить ссылку на уже уничтоженную Rust closure.
struct WebSocketHandlerCleanup {
    websocket: WebSocket,
    close_on_drop: bool,
}

impl Drop for WebSocketHandlerCleanup {
    fn drop(&mut self) {
        self.websocket.set_onopen(None);
        self.websocket.set_onmessage(None);
        self.websocket.set_onclose(None);
        self.websocket.set_onerror(None);

        if self.close_on_drop {
            let _ = self.websocket.close();
        }
    }
}

pub(in crate::features::host_settings::log_stream) async fn run(
    access_token: String,
    output: mpsc::UnboundedSender<HostLogStreamMessage>,
) -> Result<(), String> {
    let url = crate::config::host_logs_websocket_url()?.to_string();
    let websocket = WebSocket::new(&url)
        .map_err(|error| format!("Не удалось открыть поток логов: {}", js_error(error)))?;

    wait_until_open(&websocket).await?;

    let auth = serde_json::json!({ "access_token": access_token }).to_string();
    websocket
        .send_with_str(&auth)
        .map_err(|error| format!("Не удалось авторизовать поток логов: {}", js_error(error)))?;

    let message_output = output.clone();
    let message_socket = websocket.clone();
    let message_closure = Closure::wrap(Box::new(move |event: MessageEvent| {
        let Some(text) = event.data().as_string() else {
            let _ = message_output.unbounded_send(HostLogStreamMessage::Error {
                message: "Сервер прислал неподдерживаемый формат записи журнала.".to_owned(),
                retryable: true,
            });
            let _ = message_socket.close();
            return;
        };

        match serde_json::from_str::<HostLogStreamMessage>(&text) {
            Ok(message) => {
                if message_output.unbounded_send(message).is_err() {
                    let _ = message_socket.close();
                }
            }
            Err(_) => {
                let _ = message_output.unbounded_send(HostLogStreamMessage::Error {
                    message: "Не удалось прочитать запись журнала от сервера.".to_owned(),
                    retryable: true,
                });
                let _ = message_socket.close();
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    let (close_sender, close_receiver) = oneshot::channel::<()>();
    let close_sender = Rc::new(RefCell::new(Some(close_sender)));

    let close_state = close_sender.clone();
    let close_closure = Closure::wrap(Box::new(move |_event: Event| {
        if let Some(sender) = close_state.borrow_mut().take() {
            let _ = sender.send(());
        }
    }) as Box<dyn FnMut(Event)>);

    let error_state = close_sender;
    let error_closure = Closure::wrap(Box::new(move |_event: Event| {
        if let Some(sender) = error_state.borrow_mut().take() {
            let _ = sender.send(());
        }
    }) as Box<dyn FnMut(Event)>);

    websocket.set_onmessage(Some(message_closure.as_ref().unchecked_ref()));
    websocket.set_onclose(Some(close_closure.as_ref().unchecked_ref()));
    websocket.set_onerror(Some(error_closure.as_ref().unchecked_ref()));

    // Guard объявлен после Closure, поэтому при cancellation уничтожится первым:
    // сначала снимет callbacks, затем безопасно уничтожатся Rust Closure.
    let _cleanup = WebSocketHandlerCleanup {
        websocket: websocket.clone(),
        close_on_drop: true,
    };

    let _ = close_receiver.await;

    Err("Соединение с realtime-журналом закрыто.".to_owned())
}

async fn wait_until_open(websocket: &WebSocket) -> Result<(), String> {
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
            let _ = sender.send(Err("Не удалось подключиться к realtime-журналу.".to_owned()));
        }
    }) as Box<dyn FnMut(Event)>);

    let close_sender = sender;
    let close_closure = Closure::wrap(Box::new(move |_event: Event| {
        if let Some(sender) = close_sender.borrow_mut().take() {
            let _ = sender.send(Err(
                "Соединение закрылось до подключения к realtime-журналу.".to_owned(),
            ));
        }
    }) as Box<dyn FnMut(Event)>);

    websocket.set_onopen(Some(open_closure.as_ref().unchecked_ref()));
    websocket.set_onerror(Some(error_closure.as_ref().unchecked_ref()));
    websocket.set_onclose(Some(close_closure.as_ref().unchecked_ref()));

    // Здесь сам WebSocket закрывать нельзя: после успешного open он используется run().
    // Guard гарантирует снятие временных callbacks и при обычном выходе, и при cancellation.
    let _cleanup = WebSocketHandlerCleanup {
        websocket: websocket.clone(),
        close_on_drop: false,
    };

    let result = receiver
        .await
        .map_err(|_| "Callback подключения realtime-журнала был потерян.".to_owned())?;

    result
}

fn js_error(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}
