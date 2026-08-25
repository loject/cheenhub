//! REST-адаптер глобальных настроек хоста.

use std::time::Duration;

use axum::{
    Json, Router,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use cheenhub_contracts::rest::{
    ApiError, GmailConnectionStartResponse, HostAccessResponse, HostEmailSettingsResponse,
    HostLogStreamMessage, UpdateHostEmailSettingsRequest,
};
use serde::Deserialize;

use crate::state::AppState;

use super::application::{self, HostSettingsError};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/access", get(access))
        .route("/metrics", get(metrics))
        .route("/logs/ws", get(logs_ws))
        .route("/email", get(email_settings).patch(update_email_settings))
        .route("/email/gmail/connect", post(start_gmail_connection))
        .route("/email/gmail/callback", get(gmail_callback))
        .route("/email/gmail/disconnect", post(disconnect_gmail))
}

const HOST_LOG_AUTH_TIMEOUT: Duration = Duration::from_secs(10);
const HOST_LOG_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(25);
const HOST_LOG_SNAPSHOT_LIMIT: usize = 500;

async fn logs_ws(State(state): State<AppState>, upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.on_upgrade(move |socket| handle_logs_socket(state, socket))
}

async fn handle_logs_socket(state: AppState, mut socket: WebSocket) {
    let access_token =
        match tokio::time::timeout(HOST_LOG_AUTH_TIMEOUT, receive_log_auth(&mut socket)).await {
            Ok(Ok(access_token)) => access_token,
            Ok(Err(message)) => {
                let _ = send_host_log_message(
                    &mut socket,
                    HostLogStreamMessage::Error {
                        message,
                        retryable: false,
                    },
                )
                .await;
                return;
            }
            Err(_) => {
                let _ = send_host_log_message(
                    &mut socket,
                    HostLogStreamMessage::Error {
                        message: "Истекло время авторизации потока логов.".to_owned(),
                        retryable: true,
                    },
                )
                .await;
                return;
            }
        };

    let user_id = match application::require_host_owner(&state, &access_token).await {
        Ok(user_id) => user_id,
        Err(error) => {
            let (message, retryable) = match error {
                HostSettingsError::Forbidden(message) => (message, false),
                HostSettingsError::Unauthorized(message) => (message, true),
                HostSettingsError::BadRequest(message)
                | HostSettingsError::Misconfigured(message) => (message, true),
                HostSettingsError::Internal(_) => (
                    "Не удалось проверить доступ к журналу сервера.".to_owned(),
                    true,
                ),
            };
            let _ = send_host_log_message(
                &mut socket,
                HostLogStreamMessage::Error { message, retryable },
            )
            .await;
            return;
        }
    };

    tracing::info!(%user_id, "opened host backend log stream");

    // Подписка создаётся до snapshot, чтобы не потерять события,
    // появившиеся между чтением буфера и началом realtime-цикла.
    let mut receiver = state.host_logs.subscribe();
    let snapshot = state.host_logs.snapshot(HOST_LOG_SNAPSHOT_LIMIT);
    let mut last_sent_id = snapshot.last().map(|entry| entry.id).unwrap_or_default();

    if !send_host_log_message(
        &mut socket,
        HostLogStreamMessage::Snapshot { entries: snapshot },
    )
    .await
    {
        return;
    }

    let mut heartbeat = tokio::time::interval(HOST_LOG_HEARTBEAT_INTERVAL);
    // Первый tick interval происходит сразу; пропускаем его.
    heartbeat.tick().await;

    loop {
        tokio::select! {
            received = receiver.recv() => {
                match received {
                    Ok(entry) => {
                        if entry.id <= last_sent_id {
                            continue;
                        }
                        last_sent_id = entry.id;
                        if !send_host_log_message(
                            &mut socket,
                            HostLogStreamMessage::Entry { entry },
                        ).await {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        let snapshot = state.host_logs.snapshot(HOST_LOG_SNAPSHOT_LIMIT);
                        if let Some(entry) = snapshot.last() {
                            last_sent_id = entry.id;
                        }
                        if !send_host_log_message(
                            &mut socket,
                            HostLogStreamMessage::Snapshot { entries: snapshot },
                        ).await {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Text(_)))
                    | Some(Ok(Message::Binary(_)))
                    | Some(Ok(Message::Pong(_))) => {}
                    Some(Err(_)) => break,
                }
            }
            _ = heartbeat.tick() => {
                if socket.send(Message::Ping(Default::default())).await.is_err() {
                    break;
                }
            }
        }
    }

    tracing::info!(%user_id, "closed host backend log stream");
}

async fn receive_log_auth(socket: &mut WebSocket) -> Result<String, String> {
    loop {
        let message = socket
            .recv()
            .await
            .ok_or_else(|| "Соединение закрыто до авторизации.".to_owned())?
            .map_err(|_| "Не удалось прочитать данные авторизации.".to_owned())?;

        let auth = match message {
            Message::Text(text) => serde_json::from_slice::<HostLogAuthMessage>(text.as_bytes()),
            Message::Binary(bytes) => serde_json::from_slice::<HostLogAuthMessage>(&bytes),
            Message::Ping(payload) => {
                socket
                    .send(Message::Pong(payload))
                    .await
                    .map_err(|_| "Соединение закрыто во время авторизации.".to_owned())?;
                continue;
            }
            Message::Pong(_) => continue,
            Message::Close(_) => {
                return Err("Соединение закрыто до авторизации.".to_owned());
            }
        }
        .map_err(|_| "Некорректное сообщение авторизации.".to_owned())?;

        if auth.access_token.is_empty() {
            return Err("Отсутствует токен авторизации.".to_owned());
        }
        return Ok(auth.access_token);
    }
}

async fn send_host_log_message(socket: &mut WebSocket, message: HostLogStreamMessage) -> bool {
    let Ok(json) = serde_json::to_string(&message) else {
        return false;
    };
    socket.send(Message::Text(json.into())).await.is_ok()
}

#[derive(Deserialize)]
struct HostLogAuthMessage {
    access_token: String,
}

async fn metrics(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<cheenhub_contracts::rest::HostMetricsResponse>, HostSettingsError> {
    application::metrics(&state, bearer_token(&headers)?)
        .await
        .map(Json)
}

async fn access(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<HostAccessResponse>, HostSettingsError> {
    application::access(&state, bearer_token(&headers)?)
        .await
        .map(Json)
}

async fn email_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<HostEmailSettingsResponse>, HostSettingsError> {
    application::email_settings(&state, bearer_token(&headers)?)
        .await
        .map(Json)
}

async fn update_email_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UpdateHostEmailSettingsRequest>,
) -> Result<Json<HostEmailSettingsResponse>, HostSettingsError> {
    application::update_email_settings(&state, bearer_token(&headers)?, request)
        .await
        .map(Json)
}

async fn start_gmail_connection(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<GmailConnectionStartResponse>, HostSettingsError> {
    application::start_gmail_connection(&state, bearer_token(&headers)?)
        .await
        .map(Json)
}

async fn gmail_callback(
    State(state): State<AppState>,
    Query(query): Query<GmailCallbackQuery>,
) -> Redirect {
    Redirect::to(
        &application::complete_gmail_connection(&state, query.code, query.state, query.error).await,
    )
}

async fn disconnect_gmail(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<HostEmailSettingsResponse>, HostSettingsError> {
    application::disconnect_gmail(&state, bearer_token(&headers)?)
        .await
        .map(Json)
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, HostSettingsError> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
        .ok_or_else(|| HostSettingsError::Unauthorized("Войди, чтобы продолжить.".to_owned()))
}

impl IntoResponse for HostSettingsError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, "unauthorized", message),
            Self::Forbidden(message) => (StatusCode::FORBIDDEN, "forbidden", message),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::Misconfigured(message) => {
                (StatusCode::SERVICE_UNAVAILABLE, "misconfigured", message)
            }
            Self::Internal(error) => {
                tracing::error!(%error, "host settings request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Не удалось выполнить запрос. Попробуй еще раз.".to_owned(),
                )
            }
        };
        (
            status,
            Json(ApiError {
                code: code.to_owned(),
                message,
            }),
        )
            .into_response()
    }
}

#[derive(Deserialize)]
struct GmailCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}
