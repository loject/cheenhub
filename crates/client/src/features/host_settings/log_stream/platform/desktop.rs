//! Native WebSocket для realtime-журнала бэкенда.

use cheenhub_contracts::rest::HostLogStreamMessage;
use futures_channel::mpsc;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub(in crate::features::host_settings::log_stream) async fn run(
    access_token: String,
    output: mpsc::UnboundedSender<HostLogStreamMessage>,
) -> Result<(), String> {
    let url = crate::config::host_logs_websocket_url()?.to_string();
    let (mut websocket, _) = connect_async(url.as_str())
        .await
        .map_err(|error| format!("Не удалось открыть поток логов: {error}"))?;

    let auth = serde_json::json!({ "access_token": access_token }).to_string();
    websocket
        .send(Message::text(auth))
        .await
        .map_err(|error| format!("Не удалось авторизовать поток логов: {error}"))?;

    while let Some(message) = websocket.next().await {
        match message {
            Ok(Message::Text(text)) => {
                let message = serde_json::from_str::<HostLogStreamMessage>(text.as_str())
                    .map_err(|_| "Не удалось прочитать запись журнала от сервера.".to_owned())?;
                if output.unbounded_send(message).is_err() {
                    return Ok(());
                }
            }
            Ok(Message::Binary(bytes)) => {
                let message = serde_json::from_slice::<HostLogStreamMessage>(&bytes)
                    .map_err(|_| "Не удалось прочитать запись журнала от сервера.".to_owned())?;
                if output.unbounded_send(message).is_err() {
                    return Ok(());
                }
            }
            Ok(Message::Ping(payload)) => {
                websocket
                    .send(Message::Pong(payload))
                    .await
                    .map_err(|error| format!("Не удалось ответить серверу журнала: {error}"))?;
            }
            Ok(Message::Close(_)) => {
                return Err("Соединение с realtime-журналом закрыто.".to_owned());
            }
            Ok(_) => {}
            Err(error) => {
                return Err(format!("Ошибка realtime-журнала: {error}"));
            }
        }
    }

    Err("Соединение с realtime-журналом закрыто.".to_owned())
}
