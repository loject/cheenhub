#![warn(missing_docs)]
//! Backend entrypoint for CheenHub.

mod config;
mod db;
mod features;
mod http;
mod realtime;
mod state;
mod telemetry;

use anyhow::Context;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = config::AppConfig::from_env()?;
    telemetry::init(&config.log_filter)?;

    let address = config.socket_addr()?;
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("failed to bind backend listener at {address}"))?;
    let auth_keys = features::auth::security::keys::AuthKeys::from_config(
        &config.jwt_private_key_base64,
        config.jwt_key_id.clone(),
    )?;
    let (auth_store, server_store, text_chat_store): (
        Arc<dyn features::auth::infrastructure::AuthStore>,
        Arc<dyn features::servers::infrastructure::ServerStore>,
        Arc<dyn features::text_chat::infrastructure::TextChatStore>,
    ) = match config.auth_store {
        config::AuthStoreConfig::Postgres => {
            let database = db::connect(&config.database_url).await?;
            (
                Arc::new(features::auth::infrastructure::PostgresAuthStore::new(
                    database.clone(),
                )),
                Arc::new(features::servers::infrastructure::PostgresServerStore::new(
                    database.clone(),
                )),
                Arc::new(features::text_chat::infrastructure::PostgresTextChatStore::new(database)),
            )
        }
        config::AuthStoreConfig::InMemory => (
            Arc::new(features::auth::infrastructure::InMemoryAuthStore::default()),
            Arc::new(features::servers::infrastructure::InMemoryServerStore::default()),
            Arc::new(features::text_chat::infrastructure::InMemoryTextChatStore::default()),
        ),
    };
    let realtime_tls = realtime::ensure_tls_config(
        config.webtransport_tls_cert_path.as_deref(),
        config.webtransport_tls_key_path.as_deref(),
    )?;

    let state = state::AppState {
        auth_store,
        server_store,
        text_chat_store,
        voice_presence_store: Arc::new(
            features::voice_chat::infrastructure::InMemoryVoicePresenceStore::default(),
        ),
        realtime_hub: Arc::new(realtime::hub::RealtimeHub::default()),
        auth_keys,
        access_token_lifetime_minutes: config.access_token_lifetime_minutes,
        refresh_token_lifetime_days: config.refresh_token_lifetime_days,
        google_oauth_client_id: config.google_oauth_client_id.clone(),
        google_oauth_client_secret: config.google_oauth_client_secret.clone(),
        google_oauth_redirect_uri: config.google_oauth_redirect_uri.clone(),
        cheenhub_client_base_url: config.cheenhub_client_base_url.clone(),
        oauth_state_lifetime_minutes: config.oauth_state_lifetime_minutes,
        oauth_handoff_lifetime_minutes: config.oauth_handoff_lifetime_minutes,
        oauth_registration_lifetime_minutes: config.oauth_registration_lifetime_minutes,
    };
    let app = http::router(state.clone());
    let realtime_address = config.webtransport_socket_addr()?;
    let realtime_cert_path = realtime_tls.cert_path;
    let realtime_key_path = realtime_tls.key_path;
    let realtime_server =
        realtime::bind(realtime_address, &realtime_cert_path, &realtime_key_path)?;

    tokio::spawn(async move {
        if let Err(error) = realtime::serve(state, realtime_address, realtime_server).await {
            tracing::error!(%error, "webtransport realtime listener stopped");
        }
    });

    info!(%address, "backend listening");
    axum::serve(listener, app)
        .await
        .context("backend server stopped with an error")
}
