#![warn(missing_docs)]
//! Минимальный proxy системных метрик между Docker socket и backend CheenHub.

mod collector;
mod docker;

use std::sync::Arc;

use axum::{Router, extract::State, http::StatusCode, routing::get};
use collector::MetricsCollector;
use tokio::{net::TcpListener, sync::Mutex};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct ProxyState {
    collector: Arc<Mutex<MetricsCollector>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let socket_path =
        std::env::var("DOCKER_SOCKET_PATH").unwrap_or_else(|_| "/var/run/docker.sock".to_owned());
    let app_services = comma_separated("CHEENHUB_METRICS_APP_SERVICES", "backend,web");
    let database_service =
        std::env::var("CHEENHUB_METRICS_DATABASE_SERVICE").unwrap_or_else(|_| "db".to_owned());
    let address = std::env::var("CHEENHUB_METRICS_PROXY_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:9100".to_owned());
    let listener = TcpListener::bind(&address).await?;
    let state = ProxyState {
        collector: Arc::new(Mutex::new(MetricsCollector::new(
            socket_path,
            app_services,
            database_service,
        ))),
    };
    let app = Router::new()
        .route("/health", get(|| async { StatusCode::NO_CONTENT }))
        .route("/v1/metrics", get(metrics))
        .with_state(state);

    info!(%address, "CheenHub metrics proxy listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn metrics(
    State(state): State<ProxyState>,
) -> Result<axum::Json<cheenhub_contracts::rest::HostMetricsSample>, StatusCode> {
    match state.collector.lock().await.collect().await {
        Ok(Some(sample)) => Ok(axum::Json(sample)),
        Ok(None) => Err(StatusCode::NO_CONTENT),
        Err(error) => {
            error!(%error, "failed to collect sanitized Docker metrics");
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

fn comma_separated(key: &str, fallback: &str) -> Vec<String> {
    std::env::var(key)
        .unwrap_or_else(|_| fallback.to_owned())
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
