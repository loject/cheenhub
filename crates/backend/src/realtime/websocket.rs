//! Адаптер WebSocket-резерва для realtime.

use std::{collections::HashMap, time::Duration};

use anyhow::{Context, anyhow};
use axum::{
    extract::{
        State,
        ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use cheenhub_contracts::media::MediaDatagram;
use cheenhub_contracts::realtime::{RealtimeEnvelope, RealtimeModule};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::features::auth::application as auth_application;
use crate::state::AppState;

use super::hub::DisconnectReason;
use super::protocol::validate_envelope;
use super::sink::{DatagramSink, EnvelopeSink, WebSocketOutbound};
use super::{control, datagram, router};

const AUTHENTICATION_TIMEOUT: Duration = Duration::from_secs(10);
const OUTBOUND_QUEUE_CAPACITY: usize = 256;

/// Обновляет HTTP-запрос до соединения WebSocket-резерва для realtime.
pub(crate) async fn upgrade(
    State(state): State<AppState>,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    if crate::lifecycle::lifecycle().is_draining() {
        return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let session_id = Uuid::new_v4();
    info!(%session_id, "received WebSocket realtime fallback request");
    upgrade
        .on_upgrade(move |socket| handle_socket(state, session_id, socket))
        .into_response()
}

async fn handle_socket(state: AppState, session_id: Uuid, socket: WebSocket) {
    let lifecycle = crate::lifecycle::lifecycle();
    let Some(_session_guard) = lifecycle.track_realtime_session() else {
        let mut socket = socket;
        let _ = socket
            .send(Message::Close(Some(CloseFrame {
                code: super::SERVICE_RESTARTING_CLOSE_CODE as u16,
                reason: DisconnectReason::ServiceRestarting.close_message().into(),
            })))
            .await;
        return;
    };
    let (mut socket_sender, mut socket_receiver) = socket.split();
    let (outbound_sender, mut outbound_receiver) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
    let envelope_sink = EnvelopeSink::websocket(outbound_sender.clone());
    let writer_session_id = session_id;
    let writer = tokio::spawn(async move {
        while let Some(message) = outbound_receiver.recv().await {
            let message = match message {
                WebSocketOutbound::Envelope(envelope) => match serde_json::to_string(&envelope) {
                    Ok(json) => Message::Text(json.into()),
                    Err(error) => {
                        warn!(
                            %writer_session_id,
                            %error,
                            "failed to encode WebSocket realtime envelope"
                        );
                        continue;
                    }
                },
                WebSocketOutbound::Datagram(bytes) => Message::Binary(bytes),
                WebSocketOutbound::Close { code, reason } => {
                    // Close-кадр отправляем последним действием писателя, поэтому
                    // он всегда обрывает очередь и закрывает сокет.
                    let close = CloseFrame {
                        code: u16::try_from(code).unwrap_or(u16::MAX),
                        reason: reason.into(),
                    };
                    let _ = socket_sender.send(Message::Close(Some(close))).await;
                    break;
                }
            };

            if let Err(error) = socket_sender.send(message).await {
                debug!(
                    %writer_session_id,
                    %error,
                    "WebSocket realtime fallback writer closed"
                );
                break;
            }
        }
    });

    let mut stream_ids = HashMap::new();
    let mut last_slow_datagram_dispatch_warning_at = None;
    let result = async {
        let authentication = async {
            let envelope = read_next_envelope(&mut socket_receiver)
                .await?
                .ok_or_else(|| anyhow!("websocket closed before authentication"))?;
            control::authenticate_session(&state, &envelope_sink, envelope).await
        };
        let authentication_result = tokio::select! {
            biased;
            _ = lifecycle.wait_for_draining() => {
                queue_disconnect_close(&outbound_sender, DisconnectReason::ServiceRestarting).await;
                return Ok(());
            }
            result = timeout(AUTHENTICATION_TIMEOUT, authentication) => result,
        };
        let user = match authentication_result {
            Ok(result) => result?,
            Err(_) => {
                warn!(
                    %session_id,
                    timeout_seconds = AUTHENTICATION_TIMEOUT.as_secs(),
                    "истёк таймаут первичной аутентификации WebSocket realtime-сессии"
                );
                return Ok(());
            }
        };
        let Some(authenticated) = user else {
            info!(%session_id, "closing unauthorized WebSocket realtime fallback session");
            return Ok(());
        };
        let user = authenticated.user;
        let auth_session_id = authenticated.auth_session_id;
        let user_id = Uuid::parse_str(&user.id).context("authenticated user id is not a uuid")?;
        info!(%session_id, %user_id, %auth_session_id, "authenticated WebSocket realtime fallback session");
        let mut disconnect = state
            .realtime_hub
            .register_session(
                session_id,
                user_id,
                auth_session_id,
                DatagramSink::websocket(outbound_sender.clone()),
            )
            .await;
        if !auth_application::auth_session_is_active(&state, &auth_session_id).await? {
            warn!(
                %session_id,
                %user_id,
                %auth_session_id,
                "closing WebSocket realtime transport whose auth session was revoked during registration"
            );
            state
                .realtime_hub
                .disconnect_auth_session(&auth_session_id)
                .await;
            return Ok(());
        }

        loop {
            let message = tokio::select! {
                biased;
                _ = lifecycle.wait_for_draining() => {
                    queue_disconnect_close(&outbound_sender, DisconnectReason::ServiceRestarting).await;
                    break;
                }
                reason = disconnect.changed() => {
                    let reason = reason
                        .ok()
                        .and_then(|()| *disconnect.borrow())
                        .unwrap_or(DisconnectReason::ServiceRestarting);
                    info!(
                        %session_id,
                        %user_id,
                        %auth_session_id,
                        ?reason,
                        "closing WebSocket realtime transport after disconnect request"
                    );
                    // Close-кадр уходит через писателя: если очередь закрыта,
                    // соединение завершится вместе с сокетом.
                    queue_disconnect_close(&outbound_sender, reason).await;
                    break;
                }
                message = socket_receiver.next() => message,
            };
            let Some(message) = message else {
                break;
            };
            match message.context("failed to read WebSocket realtime message")? {
                Message::Text(text) => {
                    let envelope = serde_json::from_slice::<RealtimeEnvelope>(text.as_bytes())
                        .context("failed to decode WebSocket realtime envelope")?;
                    handle_envelope(
                        &state,
                        &user,
                        &user_id,
                        session_id,
                        &envelope_sink,
                        &mut stream_ids,
                        envelope,
                    )
                    .await?;
                }
                Message::Binary(bytes) => match MediaDatagram::decode(&bytes) {
                    Ok(datagram) => {
                        datagram::dispatch_with_warnings(
                            &state,
                            session_id,
                            user_id,
                            datagram,
                            &mut last_slow_datagram_dispatch_warning_at,
                        )
                        .await;
                    }
                    Err(error) => {
                        debug!(
                            %session_id,
                            %user_id,
                            %error,
                            bytes = bytes.len(),
                            "dropping invalid WebSocket fallback media datagram"
                        );
                    }
                },
                Message::Close(_) => break,
                Message::Ping(_) | Message::Pong(_) => {}
            }
        }

        Ok::<(), anyhow::Error>(())
    }
    .await;

    cleanup_streams(&state, session_id, &stream_ids).await;
    if let Err(error) = result {
        warn!(
            %session_id,
            %error,
            "WebSocket realtime fallback session ended with error"
        );
    }
    state.realtime_hub.unregister_session(session_id).await;
    drop(envelope_sink);
    drop(outbound_sender);
    if let Err(error) = writer.await {
        debug!(
            %session_id,
            %error,
            "WebSocket realtime fallback writer task ended unexpectedly"
        );
    }
}

async fn read_next_envelope(
    socket_receiver: &mut futures_util::stream::SplitStream<WebSocket>,
) -> anyhow::Result<Option<RealtimeEnvelope>> {
    while let Some(message) = socket_receiver.next().await {
        match message.context("failed to read WebSocket realtime authentication message")? {
            Message::Text(text) => {
                return serde_json::from_slice(text.as_bytes())
                    .map(Some)
                    .context("failed to decode WebSocket realtime authentication envelope");
            }
            Message::Binary(bytes) => {
                return serde_json::from_slice(&bytes)
                    .map(Some)
                    .context("failed to decode WebSocket realtime authentication envelope");
            }
            Message::Close(_) => return Ok(None),
            Message::Ping(_) | Message::Pong(_) => {}
        }
    }

    Ok(None)
}

async fn handle_envelope(
    state: &AppState,
    user: &cheenhub_contracts::rest::AuthUser,
    user_id: &Uuid,
    session_id: Uuid,
    send: &EnvelopeSink,
    stream_ids: &mut HashMap<RealtimeModule, Uuid>,
    envelope: RealtimeEnvelope,
) -> anyhow::Result<()> {
    validate_envelope(&envelope)?;
    let module = envelope.module;
    let stream_id = stream_id_for_module(state, user_id, send, stream_ids, module).await;
    router::dispatch(state, user, user_id, stream_id, session_id, send, envelope).await
}

async fn stream_id_for_module(
    state: &AppState,
    user_id: &Uuid,
    send: &EnvelopeSink,
    stream_ids: &mut HashMap<RealtimeModule, Uuid>,
    module: RealtimeModule,
) -> Uuid {
    if module == RealtimeModule::Control {
        return Uuid::nil();
    }
    if let Some(stream_id) = stream_ids.get(&module) {
        return *stream_id;
    }

    let stream_id = Uuid::new_v4();
    stream_ids.insert(module, stream_id);
    state
        .realtime_hub
        .register_stream(stream_id, module, *user_id, send.clone())
        .await;
    debug!(
        %stream_id,
        ?module,
        %user_id,
        "bound WebSocket fallback realtime virtual stream"
    );

    stream_id
}

async fn cleanup_streams(
    state: &AppState,
    session_id: Uuid,
    stream_ids: &HashMap<RealtimeModule, Uuid>,
) {
    for (module, stream_id) in stream_ids {
        state.realtime_hub.unregister_stream(*stream_id).await;
        router::cleanup_stream(state, *module, *stream_id).await;
        debug!(
            %session_id,
            %stream_id,
            ?module,
            "cleaned up WebSocket fallback realtime virtual stream"
        );
    }
}

// Закрытие проходит через ту же очередь, что и обычные данные сокета.
async fn queue_disconnect_close(
    sender: &mpsc::Sender<WebSocketOutbound>,
    reason: DisconnectReason,
) {
    if sender
        .send(WebSocketOutbound::Close {
            code: reason.close_code(),
            reason: reason.close_message(),
        })
        .await
        .is_err()
    {
        debug!(%reason, "WebSocket writer closed before disconnect could be queued");
    }
}

#[cfg(test)]
mod tests;
