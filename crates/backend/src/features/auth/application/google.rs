//! Вспомогательные функции интеграции с Google OAuth.

use std::{
    sync::OnceLock,
    time::{Duration as StdDuration, Instant},
};

use anyhow::Context;
use reqwest::header::CACHE_CONTROL;
use serde::Deserialize;
use tokio::sync::Mutex;
use tracing::info;
use url::Url;

use crate::features::auth::error::AuthError;
use crate::features::auth::security::google_id_token::{self, GoogleJwks};
use crate::features::auth::validation;
use crate::state::AppState;

const GOOGLE_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const GOOGLE_JWKS_DEFAULT_TTL: StdDuration = StdDuration::from_secs(60 * 60);
const GOOGLE_JWKS_MIN_TTL: StdDuration = StdDuration::from_secs(5 * 60);
const GOOGLE_JWKS_MAX_TTL: StdDuration = StdDuration::from_secs(24 * 60 * 60);
const GOOGLE_JWKS_MISSING_KEY_REFRESH_INTERVAL: StdDuration = StdDuration::from_secs(60);
static GOOGLE_JWKS_CACHE: OnceLock<GoogleJwksCache> = OnceLock::new();

struct GoogleJwksCache {
    client: reqwest::Client,
    cached: Mutex<Option<CachedGoogleJwks>>,
}

struct CachedGoogleJwks {
    keys: GoogleJwks,
    refreshed_at: Instant,
    expires_at: Instant,
}

impl GoogleJwksCache {
    fn global() -> &'static Self {
        GOOGLE_JWKS_CACHE.get_or_init(|| Self {
            client: reqwest::Client::new(),
            cached: Mutex::new(None),
        })
    }

    async fn keys_for(&self, key_id: &str) -> anyhow::Result<GoogleJwks> {
        let mut cached = self.cached.lock().await;
        if let Some(entry) = cached.as_ref()
            && entry.expires_at > Instant::now()
            && (entry.keys.contains_kid(key_id)
                || entry.refreshed_at.elapsed() < GOOGLE_JWKS_MISSING_KEY_REFRESH_INTERVAL)
        {
            return Ok(entry.keys.clone());
        }

        let response = self
            .client
            .get(GOOGLE_JWKS_URL)
            .send()
            .await
            .context("failed to load google jwks")?;
        if !response.status().is_success() {
            anyhow::bail!("google jwks endpoint returned {}", response.status());
        }
        let ttl = cache_ttl(response.headers().get(CACHE_CONTROL));
        let keys = response
            .json::<GoogleJwks>()
            .await
            .context("failed to decode google jwks")?;
        info!(
            key_count = keys.len(),
            ttl_seconds = ttl.as_secs(),
            requested_key_found = keys.contains_kid(key_id),
            "refreshed google jwks cache"
        );
        *cached = Some(CachedGoogleJwks {
            keys: keys.clone(),
            refreshed_at: Instant::now(),
            expires_at: Instant::now() + ttl,
        });

        Ok(keys)
    }
}

fn cache_ttl(value: Option<&reqwest::header::HeaderValue>) -> StdDuration {
    let max_age = value
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .split(',')
                .map(str::trim)
                .find_map(|part| part.strip_prefix("max-age="))
        })
        .and_then(|seconds| seconds.parse::<u64>().ok())
        .map(StdDuration::from_secs)
        .unwrap_or(GOOGLE_JWKS_DEFAULT_TTL);
    max_age.clamp(GOOGLE_JWKS_MIN_TTL, GOOGLE_JWKS_MAX_TTL)
}

/// Конфигурация клиента Google OAuth.
#[derive(Debug, Clone)]
pub(super) struct GoogleConfig {
    /// Идентификатор клиента Google OAuth.
    pub(super) client_id: String,
    /// Секрет клиента Google OAuth.
    pub(super) client_secret: String,
    /// Зарегистрированный callback URL бэкенда.
    pub(super) redirect_uri: String,
}

/// Подтвержденная личность Google.
#[derive(Debug, Clone)]
pub(super) struct GoogleIdentity {
    /// Стабильный subject Google.
    pub(super) subject: String,
    /// Подтвержденный email Google.
    pub(super) email: String,
    /// Отображаемое имя Google.
    pub(super) display_name: Option<String>,
}

/// Загружает конфигурацию Google OAuth из состояния приложения.
pub(super) fn google_config(state: &AppState) -> Result<GoogleConfig, AuthError> {
    let mut missing = Vec::new();
    if state.google_oauth_client_id.is_none() {
        missing.push("GOOGLE_OAUTH_CLIENT_ID");
    }
    if state.google_oauth_client_secret.is_none() {
        missing.push("GOOGLE_OAUTH_CLIENT_SECRET");
    }
    if state.google_oauth_redirect_uri.is_none() {
        missing.push("GOOGLE_OAUTH_REDIRECT_URI");
    }
    if !missing.is_empty() {
        return Err(AuthError::Misconfigured {
            feature: "google_oauth",
            missing,
            message: "Вход через Google не настроен на сервере.".to_owned(),
        });
    }

    Ok(GoogleConfig {
        client_id: state
            .google_oauth_client_id
            .clone()
            .expect("google oauth client id was checked"),
        client_secret: state
            .google_oauth_client_secret
            .clone()
            .expect("google oauth client secret was checked"),
        redirect_uri: state
            .google_oauth_redirect_uri
            .clone()
            .expect("google oauth redirect uri was checked"),
    })
}

/// Возвращает публичный Google client ID для нативной проверки ID Token.
pub(super) fn google_client_id(state: &AppState) -> Result<String, AuthError> {
    state
        .google_oauth_client_id
        .clone()
        .ok_or_else(|| AuthError::Misconfigured {
            feature: "google_native_auth",
            missing: vec!["GOOGLE_OAUTH_CLIENT_ID"],
            message: "Вход через Google не настроен на сервере.".to_owned(),
        })
}

/// Строит callback URL OAuth для фронтенда.
pub(super) fn frontend_oauth_url(state: &AppState, params: &[(&str, &str)]) -> String {
    let base = format!(
        "{}/auth/oauth/google",
        state.cheenhub_client_base_url.trim_end_matches('/')
    );
    let mut url: Url = match Url::parse(&base) {
        Ok(url) => url,
        Err(_) => return base,
    };
    for (key, value) in params {
        url.query_pairs_mut().append_pair(key, value);
    }

    url.to_string()
}

/// Обменивает authorization code и проверяет возвращенную личность Google.
pub(super) async fn exchange_google_code(
    config: &GoogleConfig,
    code: &str,
    expected_nonce: &str,
) -> Result<GoogleIdentity, AuthError> {
    let client = reqwest::Client::new();
    let token = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code),
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("redirect_uri", config.redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .context("failed to exchange google oauth code")
        .map_err(AuthError::Internal)?;
    if !token.status().is_success() {
        return Err(AuthError::Unauthorized(
            "Google не подтвердил вход. Попробуй еще раз.".to_owned(),
        ));
    }
    let token = token
        .json::<GoogleTokenResponse>()
        .await
        .context("failed to decode google oauth token response")
        .map_err(AuthError::Internal)?;
    let token_info_url = Url::parse_with_params(
        "https://oauth2.googleapis.com/tokeninfo",
        &[("id_token", token.id_token.as_str())],
    )
    .map_err(anyhow::Error::from)?;
    let token_info = client
        .get(token_info_url)
        .send()
        .await
        .context("failed to verify google id token")
        .map_err(AuthError::Internal)?;
    if !token_info.status().is_success() {
        return Err(AuthError::Unauthorized(
            "Google не подтвердил личность. Попробуй еще раз.".to_owned(),
        ));
    }
    let token_info = token_info
        .json::<GoogleTokenInfo>()
        .await
        .context("failed to decode google id token info")
        .map_err(AuthError::Internal)?;

    if token_info.aud != config.client_id
        || !matches!(
            token_info.iss.as_str(),
            "https://accounts.google.com" | "accounts.google.com"
        )
        || token_info.nonce.as_deref() != Some(expected_nonce)
        || token_info.email_verified != "true"
    {
        return Err(AuthError::Unauthorized(
            "Google не подтвердил email аккаунта.".to_owned(),
        ));
    }
    let email = token_info
        .email
        .filter(|email| validation::is_valid_email(&email.to_lowercase()))
        .ok_or_else(|| AuthError::Unauthorized("Google не вернул корректный email.".to_owned()))?;

    Ok(GoogleIdentity {
        subject: token_info.sub,
        email,
        display_name: token_info.name,
    })
}

/// Загружает актуальные Google JWK и проверяет нативный Google ID Token.
pub(super) async fn verify_google_id_token(
    client_id: &str,
    id_token: &str,
    expected_nonce: &str,
) -> Result<GoogleIdentity, AuthError> {
    let key_id = google_id_token::unverified_key_id(id_token).map_err(|_| {
        AuthError::Unauthorized("Google не подтвердил личность. Попробуй еще раз.".to_owned())
    })?;
    let jwks = GoogleJwksCache::global()
        .keys_for(&key_id)
        .await
        .map_err(AuthError::Internal)?;
    let identity = google_id_token::verify(
        id_token,
        &jwks,
        client_id,
        expected_nonce,
        chrono::Utc::now(),
    )
    .map_err(|_| {
        AuthError::Unauthorized("Google не подтвердил личность. Попробуй еще раз.".to_owned())
    })?;
    let email = identity.email.to_lowercase();
    if !validation::is_valid_email(&email) {
        return Err(AuthError::Unauthorized(
            "Google не вернул корректный email.".to_owned(),
        ));
    }

    Ok(GoogleIdentity {
        subject: identity.subject,
        email,
        display_name: identity.display_name,
    })
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    id_token: String,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenInfo {
    aud: String,
    iss: String,
    sub: String,
    email: Option<String>,
    email_verified: String,
    nonce: Option<String>,
    name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn jwks_cache_ttl_uses_bounded_cache_control_max_age() {
        assert_eq!(
            cache_ttl(Some(&HeaderValue::from_static("public, max-age=3600"))),
            StdDuration::from_secs(3600)
        );
        assert_eq!(
            cache_ttl(Some(&HeaderValue::from_static("max-age=1"))),
            GOOGLE_JWKS_MIN_TTL
        );
        assert_eq!(
            cache_ttl(Some(&HeaderValue::from_static("max-age=999999"))),
            GOOGLE_JWKS_MAX_TTL
        );
    }
}
