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
    /// Public REST API base URL used for generated asset links.
    pub(crate) cheenhub_api_base_url: String,
    /// OAuth state lifetime in minutes.
    pub(crate) oauth_state_lifetime_minutes: i64,
    /// OAuth handoff lifetime in minutes.
    pub(crate) oauth_handoff_lifetime_minutes: i64,
    /// OAuth registration intent lifetime in minutes.
    pub(crate) oauth_registration_lifetime_minutes: i64,
    /// SMTP host used for password reset emails.
    pub(crate) smtp_host: Option<String>,
    /// SMTP port used for password reset emails.
    pub(crate) smtp_port: u16,
    /// SMTP username used for password reset emails.
    pub(crate) smtp_username: Option<String>,
    /// SMTP password used for password reset emails.
    pub(crate) smtp_password: Option<String>,
    /// Sender email address for password reset emails.
    pub(crate) smtp_from_email: Option<String>,
    /// Password reset token lifetime in minutes.
    pub(crate) password_reset_token_lifetime_minutes: i64,
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
    /// Optional S3-compatible object storage configuration for chat images.
    pub(crate) chat_images_s3: Option<S3Config>,
}

/// S3-compatible object storage configuration.
#[derive(Debug, Clone)]
pub(crate) struct S3Config {
    /// S3 API endpoint URL.
    pub(crate) endpoint: String,
    /// S3 signing region.
    pub(crate) region: String,
    /// Bucket used to store chat image objects.
    pub(crate) bucket: String,
    /// Access key id.
    pub(crate) access_key_id: String,
    /// Secret access key.
    pub(crate) secret_access_key: String,
    /// Whether to force path-style addressing.
    pub(crate) force_path_style: bool,
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
            cheenhub_api_base_url: optional("CHEENHUB_API_BASE_URL", "http://127.0.0.1:3000/api"),
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
            smtp_host: env::var("SMTP_HOST").ok(),
            smtp_port: optional("SMTP_PORT", "587")
                .parse()
                .context("SMTP_PORT must be a valid u16 port")?,
            smtp_username: env::var("SMTP_USERNAME").ok(),
            smtp_password: env::var("SMTP_PASSWORD").ok(),
            smtp_from_email: env::var("SMTP_FROM_EMAIL").ok(),
            password_reset_token_lifetime_minutes: optional_positive_i64(
                "PASSWORD_RESET_TOKEN_LIFETIME_MINUTES",
                30,
            )?,
            auth_store: auth_store_config(&optional("AUTH_STORE", "postgres"))?,
            webtransport_host: optional("WEBTRANSPORT_HOST", "127.0.0.1"),
            webtransport_port: optional("WEBTRANSPORT_PORT", "4443")
                .parse()
                .context("WEBTRANSPORT_PORT must be a valid u16 port")?,
            webtransport_tls_cert_path: env::var("WEBTRANSPORT_TLS_CERT_PATH").ok(),
            webtransport_tls_key_path: env::var("WEBTRANSPORT_TLS_KEY_PATH").ok(),
            chat_images_s3: optional_s3_config()?,
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

fn optional_s3_config() -> anyhow::Result<Option<S3Config>> {
    let keys = [
        "CHAT_IMAGES_S3_ENDPOINT",
        "CHAT_IMAGES_S3_REGION",
        "CHAT_IMAGES_S3_BUCKET",
        "CHAT_IMAGES_S3_ACCESS_KEY_ID",
        "CHAT_IMAGES_S3_SECRET_ACCESS_KEY",
    ];
    let present = keys
        .iter()
        .filter(|key| {
            env::var(key)
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
        })
        .count();
    if present == 0 {
        return Ok(None);
    }
    if present != keys.len() {
        return Err(anyhow!(
            "chat image S3 storage is partially configured; set all of {}",
            keys.join(", ")
        ));
    }

    Ok(Some(S3Config {
        endpoint: required("CHAT_IMAGES_S3_ENDPOINT")?,
        region: required("CHAT_IMAGES_S3_REGION")?,
        bucket: required("CHAT_IMAGES_S3_BUCKET")?,
        access_key_id: required("CHAT_IMAGES_S3_ACCESS_KEY_ID")?,
        secret_access_key: required("CHAT_IMAGES_S3_SECRET_ACCESS_KEY")?,
        force_path_style: optional_bool("CHAT_IMAGES_S3_FORCE_PATH_STYLE", true)?,
    }))
}

fn optional_bool(key: &str, default: bool) -> anyhow::Result<bool> {
    let value = env::var(key).unwrap_or_else(|_| default.to_string());
    match value.trim().to_lowercase().as_str() {
        "true" | "1" | "yes" | "y" => Ok(true),
        "false" | "0" | "no" | "n" => Ok(false),
        _ => Err(anyhow!("{key} must be a boolean")),
    }
}
