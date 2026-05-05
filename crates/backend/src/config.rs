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
    /// Base64 encoded Ed25519 private key seed used to sign access JWTs.
    pub(crate) jwt_private_key_base64: String,
    /// Active JWT key identifier.
    pub(crate) jwt_key_id: String,
    /// Access JWT lifetime in minutes.
    pub(crate) access_token_lifetime_minutes: i64,
    /// Refresh token lifetime in days.
    pub(crate) refresh_token_lifetime_days: i64,
    /// Authentication storage backend.
    pub(crate) auth_store: AuthStoreConfig,
}

/// Authentication storage backend configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthStoreConfig {
    /// Store auth state in Postgres.
    Postgres,
    /// Store auth state in process memory.
    InMemory,
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
            jwt_private_key_base64: required("JWT_ED25519_PRIVATE_KEY_BASE64")?,
            jwt_key_id: required("JWT_KEY_ID")?,
            access_token_lifetime_minutes: positive_i64("ACCESS_TOKEN_LIFETIME_MINUTES")?,
            refresh_token_lifetime_days: positive_i64("REFRESH_TOKEN_LIFETIME_DAYS")?,
            auth_store: auth_store_config(&optional("AUTH_STORE", "postgres"))?,
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

fn positive_i64(key: &str) -> anyhow::Result<i64> {
    let value = required(key)?;
    let parsed = value
        .parse()
        .with_context(|| format!("{key} must be a valid i64"))?;
    if parsed <= 0 {
        return Err(anyhow!("{key} must be greater than zero"));
    }

    Ok(parsed)
}

fn auth_store_config(value: &str) -> anyhow::Result<AuthStoreConfig> {
    match value.trim().to_lowercase().as_str() {
        "postgres" => Ok(AuthStoreConfig::Postgres),
        "inmemory" | "in-memory" => Ok(AuthStoreConfig::InMemory),
        _ => Err(anyhow!("AUTH_STORE must be either postgres or inmemory")),
    }
}
