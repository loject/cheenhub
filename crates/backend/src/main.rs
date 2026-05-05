#![warn(missing_docs)]
//! Backend entrypoint for CheenHub.

mod config;
mod db;
mod features;
mod http;
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
    let (auth_store, server_store): (
        Arc<dyn features::auth::infrastructure::AuthStore>,
        Arc<dyn features::servers::infrastructure::ServerStore>,
    ) = match config.auth_store {
        config::AuthStoreConfig::Postgres => {
            let database = db::connect(&config.database_url).await?;
            (
                Arc::new(features::auth::infrastructure::PostgresAuthStore::new(
                    database.clone(),
                )),
                Arc::new(features::servers::infrastructure::PostgresServerStore::new(
                    database,
                )),
            )
        }
        config::AuthStoreConfig::InMemory => (
            Arc::new(features::auth::infrastructure::InMemoryAuthStore::default()),
            Arc::new(features::servers::infrastructure::InMemoryServerStore::default()),
        ),
    };
    let state = http::AppState {
        auth_store,
        server_store,
        auth_keys,
        access_token_lifetime_minutes: config.access_token_lifetime_minutes,
        refresh_token_lifetime_days: config.refresh_token_lifetime_days,
    };
    let app = http::router(state);

    info!(%address, "backend listening");
    axum::serve(listener, app)
        .await
        .context("backend server stopped with an error")
}
