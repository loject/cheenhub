//! Environment-backed backend configuration.

use std::{env, net::SocketAddr};

use anyhow::{Context, anyhow};

/// Runtime configuration for the backend service.
#[derive(Debug, Clone)]
pub(crate) struct AppConfig {
    /// Postgres connection string.
    pub(crate) database_url: String,
    /// Host address used by the HTTP listener.
    pub(crate) backend_host: String,
    /// Port used by the HTTP listener.
    pub(crate) backend_port: u16,
    /// Tracing filter used by `tracing-subscriber`.
    pub(crate) log_filter: String,
}

impl AppConfig {
    /// Loads backend configuration from the process environment.
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            backend_host: optional("BACKEND_HOST", "127.0.0.1"),
            backend_port: optional("BACKEND_PORT", "3000")
                .parse()
                .context("BACKEND_PORT must be a valid u16 port")?,
            log_filter: optional("RUST_LOG", "cheenhub_backend=debug,tower_http=debug,info"),
        })
    }

    /// Returns the socket address used by the HTTP listener.
    pub(crate) fn socket_addr(&self) -> anyhow::Result<SocketAddr> {
        format!("{}:{}", self.backend_host, self.backend_port)
            .parse()
            .with_context(|| {
                format!(
                    "BACKEND_HOST and BACKEND_PORT must form a valid socket address: {}:{}",
                    self.backend_host, self.backend_port
                )
            })
    }
}

fn required(key: &str) -> anyhow::Result<String> {
    env::var(key).map_err(|_| anyhow!("missing required environment variable {key}"))
}

fn optional(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}
