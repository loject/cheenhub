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
mod tls;
#[cfg(test)]
mod tls_integration;
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
pub(crate) use tls::ensure_tls_config;

const REALTIME_PATH: &str = "/realtime";

/// Привязывает слушатель realtime WebTransport.
pub(crate) fn bind(address: SocketAddr, cert_path: &str, key_path: &str) -> anyhow::Result<Server> {
    let config = tls::build_server_config(cert_path, key_path)?;
    let endpoint = quinn::Endpoint::server(config, address)
        .context("failed to bind WebTransport UDP listener")?;
    Ok(Server::new(endpoint))
}

/// Обслуживает принятые realtime-сессии WebTransport.
pub(crate) async fn serve(
    state: AppState,
    address: SocketAddr,
    mut server: Server,
    tls: tls::TlsConfig,
    reload_interval_seconds: u64,
) -> anyhow::Result<()> {
    info!(%address, "webtransport realtime listening");
    let endpoint = std::ops::Deref::deref(&server).clone();
    let watcher = tls_reload::spawn_tls_reloader(endpoint, tls, reload_interval_seconds);

    while let Some(request) = server.accept().await {
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

        let state = state.clone();
        tokio::spawn(async move {
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
