//! WebTransport realtime server scaffold.

mod control;
mod framing;
pub(crate) mod hub;
mod network;
pub(crate) mod protocol;
mod router;
mod session;
mod tls;

use std::net::SocketAddr;

use anyhow::Context;
use tracing::{info, warn};
use uuid::Uuid;
use web_transport::Session;
use web_transport_quinn::{Server, ServerBuilder};

use crate::state::AppState;

pub(crate) use tls::ensure_tls_config;

const REALTIME_PATH: &str = "/realtime";

/// Binds the WebTransport realtime listener.
pub(crate) fn bind(address: SocketAddr, cert_path: &str, key_path: &str) -> anyhow::Result<Server> {
    let certificates = tls::load_certificates(cert_path)?;
    let private_key = tls::load_private_key(key_path)?;
    ServerBuilder::new()
        .with_addr(address)
        .with_certificate(certificates, private_key)
        .context("failed to build WebTransport server")
}

/// Serves accepted WebTransport realtime sessions.
pub(crate) async fn serve(
    state: AppState,
    address: SocketAddr,
    mut server: Server,
) -> anyhow::Result<()> {
    info!(%address, "webtransport realtime listening");

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

    Ok(())
}
