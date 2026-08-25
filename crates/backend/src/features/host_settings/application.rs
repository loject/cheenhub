//! Сценарии глобальных настроек хоста.

use std::time::Duration;

use cheenhub_contracts::rest::{
    EmailTransport as ContractEmailTransport, GmailConnectionStartResponse, HostAccessResponse,
    HostEmailSettingsResponse, HostMetricsResponse, UpdateHostEmailSettingsRequest,
};
use chrono::{Duration as ChronoDuration, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::features::auth::application::require_current_user;
use crate::features::auth::error::AuthError;
use crate::features::auth::security::refresh_token;
use crate::state::AppState;

use super::domain::{EmailTransport, GmailOAuthState, HostEmailSettings};

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

/// Ошибка управления настройками хоста.
#[derive(Debug)]
pub(crate) enum HostSettingsError {
    Unauthorized(String),
    Forbidden(String),
    BadRequest(String),
    Misconfigured(String),
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for HostSettingsError {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value)
    }
}

impl From<reqwest::Error> for HostSettingsError {
    fn from(value: reqwest::Error) -> Self {
        Self::Internal(value.into())
    }
}

impl From<url::ParseError> for HostSettingsError {
    fn from(value: url::ParseError) -> Self {
        Self::Internal(value.into())
    }
}

/// Проверяет, является ли текущий пользователь владельцем хоста.
pub(crate) async fn access(
    state: &AppState,
    access_token: &str,
) -> Result<HostAccessResponse, HostSettingsError> {
    let (user, _) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    Ok(HostAccessResponse {
        is_host_owner: state.host_settings_store.is_host_owner(user.id).await?,
    })
}

async fn require_host_owner(
    state: &AppState,
    access_token: &str,
) -> Result<Uuid, HostSettingsError> {
    let (user, _) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    if !state.host_settings_store.is_host_owner(user.id).await? {
        tracing::warn!(user_id = %user.id, "rejected host settings access by non-owner");
        return Err(HostSettingsError::Forbidden(
            "Настройки хоста доступны только владельцу хоста.".to_owned(),
        ));
    }
    Ok(user.id)
}

/// Возвращает настройки почты с удалёнными секретами.
pub(crate) async fn email_settings(
    state: &AppState,
    access_token: &str,
) -> Result<HostEmailSettingsResponse, HostSettingsError> {
    require_host_owner(state, access_token).await?;
    Ok(redact(
        state,
        state.host_settings_store.load_email_settings().await?,
    ))
}

/// Возвращает последние измерения нагрузки только владельцу хоста.
pub(crate) async fn metrics(
    state: &AppState,
    access_token: &str,
) -> Result<HostMetricsResponse, HostSettingsError> {
    let user_id = require_host_owner(state, access_token).await?;
    let response = state.host_metrics.snapshot().await;
    tracing::debug!(
        %user_id,
        sample_count = response.samples.len(),
        metrics_available = response.available,
        "returned host metrics dashboard snapshot"
    );
    Ok(response)
}

/// Обновляет настройки почты без раскрытия или случайной очистки секретов.
pub(crate) async fn update_email_settings(
    state: &AppState,
    access_token: &str,
    request: UpdateHostEmailSettingsRequest,
) -> Result<HostEmailSettingsResponse, HostSettingsError> {
    let user_id = require_host_owner(state, access_token).await?;
    let mut settings = state.host_settings_store.load_email_settings().await?;

    if let Some(transport) = request.transport {
        settings.transport = match transport {
            ContractEmailTransport::Smtp => EmailTransport::Smtp,
            ContractEmailTransport::GmailApi => EmailTransport::GmailApi,
        };
    }
    if let Some(timeout) = request.email_send_timeout_seconds {
        if !(1..=300).contains(&timeout) {
            return Err(HostSettingsError::BadRequest(
                "Таймаут отправки должен быть от 1 до 300 секунд.".to_owned(),
            ));
        }
        settings.email_send_timeout_seconds = timeout;
    }
    if let Some(port) = request.smtp_port {
        if port == 0 {
            return Err(HostSettingsError::BadRequest(
                "Порт SMTP должен быть от 1 до 65535.".to_owned(),
            ));
        }
        settings.smtp_port = port;
    }
    replace_optional(&mut settings.smtp_host, request.smtp_host);
    replace_optional(&mut settings.smtp_username, request.smtp_username);
    replace_optional(&mut settings.smtp_from_email, request.smtp_from_email);
    update_gmail_client_id(
        &mut settings.gmail_client_id,
        request.gmail_client_id,
        state.google_oauth_client_id.as_ref(),
    );
    update_secret(
        &mut settings.smtp_password,
        request.smtp_password,
        request.clear_smtp_password.unwrap_or(false),
    );
    update_secret(
        &mut settings.gmail_client_secret,
        request.gmail_client_secret,
        request.clear_gmail_client_secret.unwrap_or(false),
    );

    let settings = state
        .host_settings_store
        .save_email_settings(settings, user_id, Utc::now())
        .await?;
    tracing::info!(%user_id, email_transport = settings.transport.as_str(), "updated host email settings");
    Ok(redact(state, settings))
}

/// Создаёт одноразовое состояние и URL подключения Gmail.
pub(crate) async fn start_gmail_connection(
    state: &AppState,
    access_token: &str,
) -> Result<GmailConnectionStartResponse, HostSettingsError> {
    let user_id = require_host_owner(state, access_token).await?;
    let settings = effective_gmail_oauth_settings(
        state,
        state.host_settings_store.load_email_settings().await?,
    );
    let client_id = required(
        settings.gmail_client_id,
        "Сначала укажи OAuth Client ID Gmail.",
    )?;
    required(
        settings.gmail_client_secret,
        "Сначала укажи OAuth Client Secret Gmail.",
    )?;
    let raw_state = refresh_token::generate();
    let now = Utc::now();
    state
        .host_settings_store
        .insert_gmail_oauth_state(GmailOAuthState {
            id: Uuid::new_v4(),
            state_hash: refresh_token::hash(&raw_state),
            user_id,
            created_at: now,
            expires_at: now + ChronoDuration::minutes(10),
        })
        .await?;
    let redirect_uri = gmail_callback_uri(state);
    let authorization_url = url::Url::parse_with_params(
        GOOGLE_AUTH_URL,
        [
            ("client_id", client_id.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("response_type", "code"),
            (
                "scope",
                "openid email https://www.googleapis.com/auth/gmail.send",
            ),
            ("access_type", "offline"),
            ("prompt", "consent"),
            ("state", raw_state.as_str()),
        ],
    )?;
    tracing::info!(%user_id, "started host Gmail OAuth connection");
    Ok(GmailConnectionStartResponse {
        authorization_url: authorization_url.into(),
    })
}

/// Завершает callback Gmail, сохраняет refresh token и возвращает URL клиента.
pub(crate) async fn complete_gmail_connection(
    state: &AppState,
    code: Option<String>,
    raw_state: Option<String>,
    provider_error: Option<String>,
) -> String {
    match complete_gmail_connection_inner(state, code, raw_state, provider_error).await {
        Ok(email) => redirect_result(state, "connected", Some(&email)),
        Err(error) => {
            tracing::warn!(?error, "failed to complete host Gmail OAuth connection");
            redirect_result(state, "error", None)
        }
    }
}

async fn complete_gmail_connection_inner(
    state: &AppState,
    code: Option<String>,
    raw_state: Option<String>,
    provider_error: Option<String>,
) -> Result<String, HostSettingsError> {
    if provider_error.is_some() {
        return Err(HostSettingsError::BadRequest(
            "Google отклонил подключение.".to_owned(),
        ));
    }
    let code = required(code, "Google не вернул код авторизации.")?;
    let raw_state = required(raw_state, "Отсутствует состояние OAuth.")?;
    let now = Utc::now();
    let user_id = state
        .host_settings_store
        .consume_gmail_oauth_state(&refresh_token::hash(&raw_state), now)
        .await?
        .ok_or_else(|| {
            HostSettingsError::BadRequest(
                "Состояние OAuth истекло или уже использовано.".to_owned(),
            )
        })?;
    if !state.host_settings_store.is_host_owner(user_id).await? {
        return Err(HostSettingsError::Forbidden(
            "Права владельца хоста отозваны.".to_owned(),
        ));
    }
    let mut settings = state.host_settings_store.load_email_settings().await?;
    let effective_settings = effective_gmail_oauth_settings(state, settings.clone());
    let client_id = required(
        effective_settings.gmail_client_id,
        "OAuth Client ID Gmail не настроен.",
    )?;
    let client_secret = required(
        effective_settings.gmail_client_secret,
        "OAuth Client Secret Gmail не настроен.",
    )?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(settings.email_send_timeout_seconds))
        .build()?;
    let response = client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", gmail_callback_uri(state).as_str()),
        ])
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Gmail OAuth token exchange failed with HTTP {}",
            response.status()
        )
        .into());
    }
    let tokens: GmailTokenResponse = response.json().await?;
    let refresh_token = required(tokens.refresh_token, "Google не вернул refresh token.")?;
    let profile_response = client
        .get(GOOGLE_USERINFO_URL)
        .bearer_auth(&tokens.access_token)
        .send()
        .await?;
    if !profile_response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Google user info failed with HTTP {}",
            profile_response.status()
        )
        .into());
    }
    let profile: GoogleProfile = profile_response.json().await?;
    if !profile.verified_email.unwrap_or(false) {
        return Err(HostSettingsError::BadRequest(
            "Google email не подтверждён.".to_owned(),
        ));
    }
    settings.gmail_refresh_token = Some(refresh_token);
    settings.gmail_from_email = Some(profile.email.clone());
    state
        .host_settings_store
        .save_email_settings(settings, user_id, now)
        .await?;
    tracing::info!(%user_id, gmail_email = %profile.email, "connected host Gmail sender");
    Ok(profile.email)
}

/// Отключает Gmail и удаляет сохранённый refresh token и адрес.
pub(crate) async fn disconnect_gmail(
    state: &AppState,
    access_token: &str,
) -> Result<HostEmailSettingsResponse, HostSettingsError> {
    let user_id = require_host_owner(state, access_token).await?;
    let mut settings = state.host_settings_store.load_email_settings().await?;
    settings.gmail_refresh_token = None;
    settings.gmail_from_email = None;
    let settings = state
        .host_settings_store
        .save_email_settings(settings, user_id, Utc::now())
        .await?;
    tracing::info!(%user_id, "disconnected host Gmail sender");
    Ok(redact(state, settings))
}

fn redact(state: &AppState, settings: HostEmailSettings) -> HostEmailSettingsResponse {
    let gmail_client_id_from_environment =
        settings.gmail_client_id.is_none() && state.google_oauth_client_id.is_some();
    let gmail_client_secret_from_environment =
        settings.gmail_client_secret.is_none() && state.google_oauth_client_secret.is_some();
    let settings = effective_gmail_oauth_settings(state, settings);
    HostEmailSettingsResponse {
        transport: match settings.transport {
            EmailTransport::Smtp => ContractEmailTransport::Smtp,
            EmailTransport::GmailApi => ContractEmailTransport::GmailApi,
        },
        email_send_timeout_seconds: settings.email_send_timeout_seconds,
        smtp_host: settings.smtp_host,
        smtp_port: settings.smtp_port,
        smtp_username: settings.smtp_username,
        smtp_password_configured: settings.smtp_password.is_some(),
        smtp_from_email: settings.smtp_from_email,
        gmail_client_id: settings.gmail_client_id,
        gmail_client_id_from_environment,
        gmail_client_secret_configured: settings.gmail_client_secret.is_some(),
        gmail_client_secret_from_environment,
        gmail_connected: settings.gmail_refresh_token.is_some(),
        gmail_from_email: settings.gmail_from_email,
        gmail_oauth_redirect_uri: gmail_callback_uri(state),
    }
}

fn replace_optional(target: &mut Option<String>, update: Option<String>) {
    if let Some(value) = update {
        *target = non_empty(value);
    }
}

fn update_gmail_client_id(
    target: &mut Option<String>,
    update: Option<String>,
    environment_fallback: Option<&String>,
) {
    let Some(update) = update else {
        return;
    };
    let update = non_empty(update);
    if target.is_none() && update.as_ref() == environment_fallback {
        return;
    }
    *target = update;
}

fn effective_gmail_oauth_settings(
    state: &AppState,
    settings: HostEmailSettings,
) -> HostEmailSettings {
    settings.with_gmail_oauth_fallback(
        state.google_oauth_client_id.clone(),
        state.google_oauth_client_secret.clone(),
    )
}

fn update_secret(target: &mut Option<String>, update: Option<String>, clear: bool) {
    if clear {
        *target = None;
    } else if let Some(value) = update.filter(|value| !value.trim().is_empty()) {
        *target = Some(value);
    }
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

fn required<T>(value: Option<T>, message: &str) -> Result<T, HostSettingsError> {
    value.ok_or_else(|| HostSettingsError::Misconfigured(message.to_owned()))
}

fn map_auth_error(error: AuthError) -> HostSettingsError {
    match error {
        AuthError::Internal(error) => HostSettingsError::Internal(error),
        _ => HostSettingsError::Unauthorized("Войди, чтобы продолжить.".to_owned()),
    }
}

fn gmail_callback_uri(state: &AppState) -> String {
    format!(
        "{}/host-settings/email/gmail/callback",
        state.cheenhub_api_base_url.trim_end_matches('/')
    )
}

fn redirect_result(state: &AppState, result: &str, email: Option<&str>) -> String {
    let mut url = url::Url::parse(&format!(
        "{}/app/host-settings/email",
        state.cheenhub_client_base_url.trim_end_matches('/')
    ))
    .expect("CHEENHUB_CLIENT_BASE_URL validated by configuration");
    url.query_pairs_mut().append_pair("gmail", result);
    if let Some(email) = email {
        url.query_pairs_mut().append_pair("email", email);
    }
    url.into()
}

#[derive(Deserialize)]
struct GmailTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct GoogleProfile {
    email: String,
    verified_email: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::{non_empty, update_gmail_client_id, update_secret};

    #[test]
    fn blank_secret_keeps_existing_value_and_explicit_clear_removes_it() {
        let mut value = Some("existing".to_owned());
        update_secret(&mut value, Some("   ".to_owned()), false);
        assert_eq!(value.as_deref(), Some("existing"));

        update_secret(&mut value, Some("replacement".to_owned()), false);
        assert_eq!(value.as_deref(), Some("replacement"));

        update_secret(&mut value, None, true);
        assert_eq!(value, None);
    }

    #[test]
    fn blank_non_secret_is_an_explicit_clear() {
        assert_eq!(non_empty("   ".to_owned()), None);
        assert_eq!(non_empty(" host ".to_owned()).as_deref(), Some("host"));
    }

    #[test]
    fn unchanged_environment_client_id_is_not_copied_into_database() {
        let environment = "environment-id".to_owned();
        let mut database = None;

        update_gmail_client_id(&mut database, Some(environment.clone()), Some(&environment));
        assert_eq!(database, None);

        update_gmail_client_id(
            &mut database,
            Some("database-id".to_owned()),
            Some(&environment),
        );
        assert_eq!(database.as_deref(), Some("database-id"));
    }
}
