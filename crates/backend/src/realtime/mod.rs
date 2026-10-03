//! Каркас сервера realtime WebTransport.

mod control;
mod datagram;
mod framing;
pub(crate) mod hub;
mod network;
pub(crate) mod protocol;
mod router;
mod session;
mod sink;
#[cfg(test)]
mod tests;
mod tls;
mod tls_reload;
pub(crate) mod websocket;

use std::net::SocketAddr;

use anyhow::Context;
use tracing::{info, warn};
use uuid::Uuid;
use web_transport::Session;
use web_transport_quinn::Server;

use crate::state::AppState;

pub(crate) use sink::EnvelopeSink;
#[cfg(test)]
pub(crate) use sink::WebSocketOutbound;
pub(crate) use tls::ensure_tls_config;

const REALTIME_PATH: &str = "/realtime";

/// Код закрытия транспорта при плановом перезапуске backend.
///
/// Значение 1013 (try again later) по RFC 6455 зарезервировано для ситуаций,
/// когда соединение закрывается из-за временной недоступности сервера. Клиент
/// может отличить эту причину от отзыва доступа.
pub(crate) const SERVICE_RESTARTING_CLOSE_CODE: u32 = 1013;

/// Привязывает слушатель realtime WebTransport.
pub(crate) fn bind(address: SocketAddr, cert_path: &str, key_path: &str) -> anyhow::Result<Server> {
    let config = tls::build_server_config(cert_path, key_path)?;
    let endpoint = quinn::Endpoint::server(config, address)
        .context("failed to bind WebTransport UDP listener")?;
    Ok(Server::new(endpoint))
}

/// Обслуживает принятые realtime-сессии WebTransport.
///
/// Слушатель перестаёт принимать новые сессии, как только процесс переходит в
/// фазу завершения, а активные сессии закрываются кодом 1013 «service
/// restarting». Перед завершением слушателя endpoint отправляет закрытие
/// активным QUIC-соединениям.
pub(crate) async fn serve(
    state: AppState,
    address: SocketAddr,
    mut server: Server,
    tls: tls::TlsConfig,
    reload_interval_seconds: u64,
) -> anyhow::Result<()> {
    info!(%address, "webtransport realtime listening");
    let endpoint = std::ops::Deref::deref(&server).clone();
    let watcher = tls_reload::spawn_tls_reloader(endpoint.clone(), tls, reload_interval_seconds);
    let lifecycle = crate::lifecycle::lifecycle();
    let mut draining = lifecycle.subscribe();

    while !lifecycle.is_draining() {
        let request = tokio::select! {
            biased;
            phase = draining.changed() => {
                if phase.is_err() {
                    warn!(%address, "lifecycle phase stream ended; stopping realtime listener");
                    break;
                }
                info!(%address, "stopping realtime listener after shutdown signal");
                break;
            }
            request = server.accept() => request,
        };

        let Some(request) = request else {
            info!(%address, "webtransport realtime listener closed");
            break;
        };
        let session_id = Uuid::new_v4();
        let remote_address = request.conn().remote_address();
        let url = request.url.clone();
        info!(%session_id, %remote_address, %url, "received WebTransport request");

        if request.url.path() != REALTIME_PATH {
            warn!(
                %session_id,
                %remote_address,
                %url,
                "rejecting WebTransport request for unsupported path"
            );
            if let Err(error) = request.reject(http::StatusCode::NOT_FOUND).await {
                warn!(%session_id, %remote_address, %url, %error, "failed to reject WebTransport request");
            }
            continue;
        }

        let Some(session_guard) = lifecycle.track_realtime_session() else {
            break;
        };
        let state = state.clone();
        tokio::spawn(async move {
            let _session_guard = session_guard;
            match request.ok().await {
                Ok(session) => {
                    info!(%session_id, %remote_address, %url, "accepted WebTransport request");
                    let session = Session::from(session);
                    if let Err(error) = session::handle_session(state, session_id, session).await {
                        warn!(
                            %session_id,
                            %remote_address,
                            %url,
                            %error,
                            "WebTransport session ended with error"
                        );
                    }
                }
                Err(error) => warn!(
                    %session_id,
                    %remote_address,
                    %url,
                    %error,
                    "failed to accept WebTransport request"
                ),
            }
        });
    }

    // Новые соединения больше не принимаются, поэтому endpoint закрывается
    // целиком: это освобождает UDP-сокет и отправляет клиентам CONNECTION_CLOSE.
    endpoint.close(
        quinn::VarInt::from_u32(SERVICE_RESTARTING_CLOSE_CODE),
        b"backend is shutting down",
    );
    let disconnected = state.realtime_hub.disconnect_all_sessions().await;
    info!(
        %address,
        disconnected_sessions = disconnected,
        "closing realtime sessions before process exit"
    );

    // Дожидаемся отправки QUIC закрытия, а не только постановки его в очередь.
    endpoint.wait_idle().await;

    watcher.abort();
    match watcher.await {
        Err(error) if error.is_cancelled() => {
            info!(%address, "WebTransport TLS reload watcher cancelled with listener");
        }
        Err(error) => {
            tracing::error!(%address, %error, "WebTransport TLS reload watcher task failed")
        }
        Ok(Err(_)) => {}
        Ok(Ok(())) => warn!(%address, "WebTransport TLS reload watcher stopped unexpectedly"),
    }
    Ok(())
}
