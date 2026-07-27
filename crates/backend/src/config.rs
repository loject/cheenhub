//! Конфигурация бэкенда на основе переменных окружения.

use std::{env, net::SocketAddr};

use anyhow::{Context, anyhow};
use url::Url;

/// Конфигурация сервиса бэкенда во время выполнения.
#[derive(Debug, Clone)]
pub(crate) struct AppConfig {
    /// Строка подключения к Postgres.
    pub(crate) database_url: String,
    /// Хост-адрес, используемый HTTP-слушателем.
    pub(crate) backend_host: String,
    /// Порт, используемый HTTP-слушателем.
    pub(crate) backend_port: u16,
    /// Фильтр трассировки, используемый `tracing-subscriber`.
    pub(crate) log_filter: String,
    /// Base64-кодированный seed приватного ключа Ed25519 для подписи Access JWT.
    pub(crate) jwt_private_key_base64: String,
    /// Активный идентификатор ключа Access JWT.
    pub(crate) jwt_key_id: String,
    /// Время жизни Access JWT в минутах.
    pub(crate) access_token_lifetime_minutes: i64,
    /// Время жизни Refresh токена в днях.
    pub(crate) refresh_token_lifetime_days: i64,
    /// ID клиента Google OAuth.
    pub(crate) google_oauth_client_id: Option<String>,
    /// Секрет клиента Google OAuth.
    pub(crate) google_oauth_client_secret: Option<String>,
    /// URI перенаправления Google OAuth, зарегистрированный для этого бэкенда.
    pub(crate) google_oauth_redirect_uri: Option<String>,
    /// Базовый URL браузерного клиента после обратных вызовов OAuth.
    pub(crate) cheenhub_client_base_url: String,
    /// Производный публичный базовый URL REST API для сгенерированных ссылок на ресурсы.
    pub(crate) cheenhub_api_base_url: String,
    /// Время жизни состояния OAuth в минутах.
    pub(crate) oauth_state_lifetime_minutes: i64,
    /// Время жизни передачи OAuth в минутах.
    pub(crate) oauth_handoff_lifetime_minutes: i64,
    /// Время жизни намерения регистрации OAuth в минутах.
    pub(crate) oauth_registration_lifetime_minutes: i64,
    /// Хост SMTP для писем сброса пароля.
    pub(crate) smtp_host: Option<String>,
    /// Порт SMTP для писем сброса пароля.
    pub(crate) smtp_port: u16,
    /// Имя пользователя SMTP для писем сброса пароля.
    pub(crate) smtp_username: Option<String>,
    /// Пароль SMTP для писем сброса пароля.
    pub(crate) smtp_password: Option<String>,
    /// Адрес электронной почты отправителя для писем сброса пароля.
    pub(crate) smtp_from_email: Option<String>,
    /// Время жизни токена сброса пароля в минутах.
    pub(crate) password_reset_token_lifetime_minutes: i64,
    /// Бэкенд хранения аутентификации.
    pub(crate) auth_store: AuthStoreConfig,
    /// Необязательный путь к PEM-сертификату, используемый слушателем WebTransport.
    pub(crate) webtransport_tls_cert_path: Option<String>,
    /// Необязательный путь к PEM-приватному ключу, используемый слушателем WebTransport.
    pub(crate) webtransport_tls_key_path: Option<String>,
    /// Необязательная конфигурация S3-совместимого объектного хранилища для изображений чата.
    pub(crate) chat_images_s3: Option<S3Config>,
    /// Путь к внешнему JSON service account для FCM HTTP v1.
    pub(crate) fcm_service_account_path: Option<String>,
}

/// Конфигурация S3-совместимого объектного хранилища.
#[derive(Debug, Clone)]
pub(crate) struct S3Config {
    /// URL эндпоинта S3 API.
    pub(crate) endpoint: String,
    /// Регион подписи S3.
    pub(crate) region: String,
    /// S3 bucket для хранения объектов изображений чата.
    pub(crate) bucket: String,
    /// ID ключа доступа.
    pub(crate) access_key_id: String,
    /// Секретный ключ доступа.
    pub(crate) secret_access_key: String,
    /// Принудительно использовать адресацию в стиле пути.
    pub(crate) force_path_style: bool,
}

/// Конфигурация бэкенда хранения аутентификации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthStoreConfig {
    /// Хранит состояние аутентификации в Postgres.
    Postgres,
    /// Хранит состояние аутентификации в памяти процесса.
    InMemory,
}

impl AppConfig {
    /// Загружает конфигурацию бэкенда из окружения процесса.
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
            cheenhub_client_base_url: optional("CHEENHUB_CLIENT_BASE_URL", "http://127.0.0.1:8081"),
            cheenhub_api_base_url: api_base_url(&optional(
                "CHEENHUB_BASE_URL",
                "http://127.0.0.1:3000",
            ))?,
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
            webtransport_tls_cert_path: env::var("WEBTRANSPORT_TLS_CERT_PATH").ok(),
            webtransport_tls_key_path: env::var("WEBTRANSPORT_TLS_KEY_PATH").ok(),
            chat_images_s3: optional_s3_config()?,
            fcm_service_account_path: env::var("FCM_SERVICE_ACCOUNT_PATH")
                .ok()
                .filter(|value| !value.trim().is_empty()),
        })
    }

    /// Возвращает socket address, используемый HTTP-слушателем.
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

fn api_base_url(base_url: &str) -> anyhow::Result<String> {
    let mut url = Url::parse(base_url)
        .with_context(|| "CHEENHUB_BASE_URL must contain a valid absolute URL")?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(anyhow!(
            "CHEENHUB_BASE_URL must use the http or https scheme"
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(anyhow!("CHEENHUB_BASE_URL must not contain credentials"));
    }
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        return Err(anyhow!(
            "CHEENHUB_BASE_URL must contain only a scheme, host, and optional port"
        ));
    }

    url.set_path("/api");
    Ok(url.to_string().trim_end_matches('/').to_owned())
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

#[cfg(test)]
mod tests {
    use super::api_base_url;

    #[test]
    fn derives_api_url_from_service_base_url() {
        assert_eq!(
            api_base_url("http://192.168.2.2:3000").expect("публичный API URL должен собираться"),
            "http://192.168.2.2:3000/api"
        );
        assert_eq!(
            api_base_url("https://cheenhub.test/").expect("публичный API URL должен собираться"),
            "https://cheenhub.test/api"
        );
    }

    #[test]
    fn rejects_base_url_with_path_credentials_or_unsupported_scheme() {
        assert!(api_base_url("https://cheenhub.test/root").is_err());
        assert!(api_base_url("https://user:secret@cheenhub.test").is_err());
        assert!(api_base_url("ftp://cheenhub.test").is_err());
    }
}
