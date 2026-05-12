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
    /// Google OAuth client id.
    pub(crate) google_oauth_client_id: Option<String>,
    /// Google OAuth client secret.
    pub(crate) google_oauth_client_secret: Option<String>,
    /// Google OAuth redirect URI registered for this backend.
    pub(crate) google_oauth_redirect_uri: Option<String>,
    /// Browser client base URL used after OAuth callbacks.
    pub(crate) cheenhub_client_base_url: String,
    /// OAuth state lifetime in minutes.
    pub(crate) oauth_state_lifetime_minutes: i64,
    /// OAuth handoff lifetime in minutes.
    pub(crate) oauth_handoff_lifetime_minutes: i64,
    /// OAuth registration intent lifetime in minutes.
    pub(crate) oauth_registration_lifetime_minutes: i64,
    /// Authentication storage backend.
    pub(crate) auth_store: AuthStoreConfig,
    /// Host address used by the WebTransport listener.
    pub(crate) webtransport_host: String,
    /// Port used by the WebTransport listener.
    pub(crate) webtransport_port: u16,
    /// Optional PEM certificate path used by the WebTransport listener.
    pub(crate) webtransport_tls_cert_path: Option<String>,
    /// Optional PEM private key path used by the WebTransport listener.
    pub(crate) webtransport_tls_key_path: Option<String>,
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
            google_oauth_client_id: env::var("GOOGLE_OAUTH_CLIENT_ID").ok(),
            google_oauth_client_secret: env::var("GOOGLE_OAUTH_CLIENT_SECRET").ok(),
            google_oauth_redirect_uri: env::var("GOOGLE_OAUTH_REDIRECT_URI").ok(),
            cheenhub_client_base_url: optional("CHEENHUB_CLIENT_BASE_URL", "http://127.0.0.1:8080"),
            oauth_state_lifetime_minutes: optional_positive_i64(
                "OAUTH_STATE_LIFETIME_MINUTES",
                10,
            )?,
            oauth_handoff_lifetime_minutes: optional_positive_i64(
                "OAUTH_HANDOFF_LIFETIME_MINUTES",
                5,
            )?,
            oauth_registration_lifetime_minutes: optional_positive_i64(
                "OAUTH_REGISTRATION_LIFETIME_MINUTES",
                15,
            )?,
            auth_store: auth_store_config(&optional("AUTH_STORE", "postgres"))?,
            webtransport_host: optional("WEBTRANSPORT_HOST", "127.0.0.1"),
            webtransport_port: optional("WEBTRANSPORT_PORT", "4443")
                .parse()
                .context("WEBTRANSPORT_PORT must be a valid u16 port")?,
            webtransport_tls_cert_path: env::var("WEBTRANSPORT_TLS_CERT_PATH").ok(),
            webtransport_tls_key_path: env::var("WEBTRANSPORT_TLS_KEY_PATH").ok(),
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

    /// Returns the socket address used by the WebTransport listener.
    pub(crate) fn webtransport_socket_addr(&self) -> anyhow::Result<SocketAddr> {
        format!("{}:{}", self.webtransport_host, self.webtransport_port)
            .parse()
            .with_context(|| {
                format!(
                    "WEBTRANSPORT_HOST and WEBTRANSPORT_PORT must form a valid socket address: {}:{}",
                    self.webtransport_host, self.webtransport_port
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

fn optional_positive_i64(key: &str, default: i64) -> anyhow::Result<i64> {
    let value = env::var(key).unwrap_or_else(|_| default.to_string());
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
