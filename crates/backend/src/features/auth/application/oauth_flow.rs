//! Запуск OAuth и безопасная маршрутизация callback.

use cheenhub_contracts::rest::{OAuthFlow, OAuthStartRequest, OAuthStartResponse};
use chrono::{Duration, Utc};
use tracing::{error, info, warn};
use url::Url;
use uuid::Uuid;

use super::google::{GoogleIdentity, exchange_google_code, frontend_oauth_url, google_config};
use super::oauth::create_google_handoff;
use super::{desktop_oauth, expired_session, me};
use crate::features::auth::domain::OAuthState;
use crate::features::auth::error::AuthError;
use crate::features::auth::security::refresh_token;
use crate::state::AppState;

const GOOGLE_PROVIDER: &str = "google";
const OAUTH_FLOW_LOGIN: &str = "login";
const OAUTH_FLOW_LINK: &str = "link";

/// Получатель результата браузерного callback.
pub(crate) enum GoogleCallbackOutcome {
    /// Браузерный клиент получает существующий OAuth redirect.
    WebRedirect(String),
    /// Настольный клиент получает результат отдельно; браузеру нужен только статус.
    Desktop {
        /// Удалось ли подтвердить личность для исходного приложения.
        success: bool,
    },
}

/// Запускает браузерный вход или привязку Google.
pub(crate) async fn start_google_oauth(
    state: &AppState,
    access_token: Option<&str>,
    request: OAuthStartRequest,
) -> Result<OAuthStartResponse, AuthError> {
    start_google_flow(state, access_token, request)
        .await
        .map(|(response, _)| response)
}

/// Запускает поток входа или привязки аккаунта через Google OAuth.
pub(super) async fn start_google_flow(
    state: &AppState,
    access_token: Option<&str>,
    request: OAuthStartRequest,
) -> Result<(OAuthStartResponse, Uuid), AuthError> {
    let config = google_config(state)?;
    let now = Utc::now();
    let (flow_kind, user_id) = match request.flow {
        OAuthFlow::Login => (OAUTH_FLOW_LOGIN.to_owned(), None),
        OAuthFlow::Link => {
            let token = access_token
                .ok_or_else(|| AuthError::Unauthorized("Войди, чтобы продолжить.".to_owned()))?;
            let user = me(state, token).await?;
            let user_id = Uuid::parse_str(&user.id).map_err(|_| expired_session())?;
            (OAUTH_FLOW_LINK.to_owned(), Some(user_id))
        }
    };

    let state_value = refresh_token::generate();
    let nonce = refresh_token::generate();
    let expires_at = now + Duration::minutes(state.oauth_state_lifetime_minutes);
    let oauth_state_id = state
        .auth_store
        .insert_oauth_state(
            refresh_token::hash(&state_value),
            nonce.clone(),
            flow_kind.clone(),
            user_id,
            now,
            expires_at,
        )
        .await
        .map_err(|error| {
            error!(
                provider = GOOGLE_PROVIDER,
                flow_kind,
                ?user_id,
                %expires_at,
                %error,
                "failed to persist google oauth state; ensure database migrations are applied and oauth_states table exists"
            );
            AuthError::Internal(error)
        })?;

    info!(
        provider = GOOGLE_PROVIDER,
        flow_kind,
        ?user_id,
        %expires_at,
        "started google oauth flow"
    );

    let mut url =
        Url::parse("https://accounts.google.com/o/oauth2/v2/auth").map_err(anyhow::Error::from)?;
    url.query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid email profile")
        .append_pair("state", &state_value)
        .append_pair("nonce", &nonce)
        .append_pair("prompt", "select_account");

    Ok((
        OAuthStartResponse {
            authorization_url: url.to_string(),
        },
        oauth_state_id,
    ))
}

/// Обрабатывает callback, не раскрывая desktop-секреты браузеру.
pub(crate) async fn google_oauth_callback(
    state: &AppState,
    code: Option<String>,
    state_value: Option<String>,
    provider_error: Option<String>,
) -> GoogleCallbackOutcome {
    let Some(state_value) = state_value else {
        return GoogleCallbackOutcome::WebRedirect(frontend_oauth_url(
            state,
            &[("error", "Google не вернул подтверждение входа.")],
        ));
    };
    let state_hash = refresh_token::hash(&state_value);
    let attempt_id = match state
        .auth_store
        .desktop_oauth_attempt_by_state_hash(&state_hash)
        .await
    {
        Ok(id) => id,
        Err(_) => {
            error!("Не удалось определить получателя Google OAuth callback");
            return GoogleCallbackOutcome::Desktop { success: false };
        }
    };
    let oauth_state = state
        .auth_store
        .consume_oauth_state(&state_hash, Utc::now())
        .await
        .map_err(AuthError::Internal)
        .and_then(|value| {
            value.ok_or_else(|| {
                AuthError::Unauthorized("Вход через Google истёк. Попробуй ещё раз.".to_owned())
            })
        });
    if let Some(attempt_id) = attempt_id {
        let success =
            desktop_oauth::callback(state, attempt_id, oauth_state, code, provider_error).await;
        return GoogleCallbackOutcome::Desktop { success };
    }
    let result = async {
        let oauth_state = oauth_state?;
        let identity = callback_identity(state, &oauth_state, code, provider_error).await?;
        create_google_handoff(
            state,
            &oauth_state.flow_kind,
            oauth_state.user_id,
            &identity,
            Utc::now(),
        )
        .await
    }
    .await;
    let url = match result {
        Ok(code) => frontend_oauth_url(state, &[("code", code.as_str())]),
        Err(error) => {
            let message = error
                .user_message()
                .unwrap_or("Не удалось войти через Google. Попробуй ещё раз.");
            warn!("Google OAuth callback завершился ошибкой");
            frontend_oauth_url(state, &[("error", message)])
        }
    };
    GoogleCallbackOutcome::WebRedirect(url)
}

/// Проверяет ответ Google после потребления одноразового state.
pub(super) async fn callback_identity(
    state: &AppState,
    oauth_state: &OAuthState,
    code: Option<String>,
    provider_error: Option<String>,
) -> Result<GoogleIdentity, AuthError> {
    if provider_error.is_some() {
        return Err(AuthError::BadRequest(
            "Вход через Google отменён или не подтверждён.".to_owned(),
        ));
    }
    let code =
        code.ok_or_else(|| AuthError::BadRequest("Google не вернул код входа.".to_owned()))?;
    let config = google_config(state)?;
    exchange_google_code(&config, &code, &oauth_state.nonce).await
}
