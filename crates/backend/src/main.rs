#![warn(missing_docs)]
//! Backend entrypoint for CheenHub.

mod config;
mod db;
mod features;
mod http;
mod telemetry;

use anyhow::Context;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = config::AppConfig::from_env()?;
    telemetry::init(&config.log_filter)?;

    let _database = db::connect(&config.database_url).await?;
    let address = config.socket_addr()?;
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("failed to bind backend listener at {address}"))?;
    let app = http::router();

    info!(%address, "backend listening");
    axum::serve(listener, app)
        .await
        .context("backend server stopped with an error")
}
