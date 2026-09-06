//! Передача результата браузерного Google OAuth настольному приложению.

use cheenhub_contracts::rest::{
    GoogleDesktopAuthPollResponse, GoogleDesktopAuthRequest, GoogleDesktopAuthStartResponse,
    OAuthCompleteRequest, OAuthStartRequest,
};
use chrono::{Duration, Utc};
use tracing::{info, warn};
use uuid::Uuid;

use super::google::GoogleIdentity;
use super::oauth::create_google_handoff;
use super::oauth_flow::{callback_identity, start_google_flow};
use crate::features::auth::domain::{
    DesktopOAuthAttempt, DesktopOAuthIdentity, DesktopOAuthStatus, OAuthHandoff, OAuthState,
};
use crate::features::auth::error::AuthError;
use crate::features::auth::security::refresh_token;
use crate::state::AppState;

const ATTEMPT_SECONDS: u64 = 300;
const POLL_SECONDS: u64 = 2;

/// Создаёт попытку входа, доступную только исходному desktop-клиенту.
pub(crate) async fn start_desktop_oauth(
    state: &AppState,
    access_token: Option<&str>,
    request: OAuthStartRequest,
) -> Result<GoogleDesktopAuthStartResponse, AuthError> {
    let (start, oauth_state_id) = start_google_flow(state, access_token, request).await?;
    let attempt_id = Uuid::new_v4();
    let poll_secret = refresh_token::generate();
    state
        .auth_store
        .insert_desktop_oauth_attempt(DesktopOAuthAttempt {
            id: attempt_id,
            oauth_state_id,
            secret_hash: refresh_token::hash(&poll_secret),
            expires_at: Utc::now() + Duration::seconds(ATTEMPT_SECONDS as i64),
        })
        .await
        .map_err(AuthError::Internal)?;
    info!(%attempt_id, provider = "google", "Создана попытка входа в desktop");
    Ok(GoogleDesktopAuthStartResponse {
        attempt_id,
        poll_secret,
        authorization_url: start.authorization_url,
        expires_in_seconds: ATTEMPT_SECONDS,
        poll_interval_seconds: POLL_SECONDS,
    })
}

/// Получает статус по секрету исходного приложения без передачи токенов браузеру.
pub(crate) async fn poll_desktop_oauth(
    state: &AppState,
    request: GoogleDesktopAuthRequest,
) -> Result<GoogleDesktopAuthPollResponse, AuthError> {
    let status = state
        .auth_store
        .desktop_oauth_status(
            &request.attempt_id,
            &refresh_token::hash(&request.poll_secret),
            Utc::now(),
        )
        .await
        .map_err(AuthError::Internal)?;
    match status {
        Some(DesktopOAuthStatus::Pending) => Ok(GoogleDesktopAuthPollResponse::Pending),
        Some(DesktopOAuthStatus::Ready) => Ok(GoogleDesktopAuthPollResponse::Ready),
        Some(DesktopOAuthStatus::Cancelled) => Ok(GoogleDesktopAuthPollResponse::Cancelled),
        Some(DesktopOAuthStatus::Expired) => Ok(GoogleDesktopAuthPollResponse::Expired),
        Some(DesktopOAuthStatus::Failed(message)) => {
            Ok(GoogleDesktopAuthPollResponse::Failed { message })
        }
        Some(DesktopOAuthStatus::Claimed) | None => {
            warn!(attempt_id = %request.attempt_id, "Отклонён опрос desktop OAuth: попытка недоступна или секрет неверен");
            Err(unavailable())
        }
    }
}

/// Отменяет попытку по секрету приложения; повторная отмена безопасна.
pub(crate) async fn cancel_desktop_oauth(
    state: &AppState,
    request: GoogleDesktopAuthRequest,
) -> Result<(), AuthError> {
    let cancelled = state
        .auth_store
        .cancel_desktop_oauth_attempt(
            &request.attempt_id,
            &refresh_token::hash(&request.poll_secret),
            Utc::now(),
        )
        .await
        .map_err(AuthError::Internal)?;
    info!(attempt_id = %request.attempt_id, cancelled, "Обработана отмена desktop OAuth");
    Ok(())
}

/// Сохраняет подтверждённую личность, не привязывая её до завершения в приложении.
pub(super) async fn finish_identity(
    state: &AppState,
    attempt_id: Uuid,
    oauth_state: &OAuthState,
    identity: GoogleIdentity,
) -> Result<bool, AuthError> {
    let kind = match oauth_state.flow_kind.as_str() {
        "login" => "desktop_login",
        "link" if oauth_state.user_id.is_some() => "desktop_link",
        _ => return Err(unavailable()),
    };
    info!(%attempt_id, oauth_state_id = %oauth_state.id, kind, "Сохранение Google-личности для desktop OAuth");
    state
        .auth_store
        .finish_desktop_oauth_attempt(
            &attempt_id,
            kind.to_owned(),
            oauth_state.user_id,
            DesktopOAuthIdentity {
                subject: identity.subject,
                email: identity.email,
                display_name: identity.display_name,
            },
            Utc::now(),
        )
        .await
        .map_err(AuthError::Internal)
}

/// Обрабатывает desktop callback после определения назначения по state.
pub(super) async fn callback(
    state: &AppState,
    attempt_id: Uuid,
    oauth_state: Result<OAuthState, AuthError>,
    code: Option<String>,
    provider_error: Option<String>,
) -> bool {
    let oauth_state = match oauth_state {
        Ok(value) => value,
        Err(_) => {
            warn!(%attempt_id, "Отклонён повторный или истёкший desktop OAuth callback");
            return false;
        }
    };
    let result = async {
        if !state
            .auth_store
            .desktop_oauth_attempt_is_pending(&attempt_id, Utc::now())
            .await
            .map_err(AuthError::Internal)?
        {
            return Ok(false);
        }
        let identity = callback_identity(state, &oauth_state, code, provider_error).await?;
        finish_identity(state, attempt_id, &oauth_state, identity).await
    }
    .await;
    match result {
        Ok(success) => {
            info!(%attempt_id, success, "Обработан desktop OAuth callback");
            success
        }
        Err(error) => {
            // В ошибках HTTP-клиента могут находиться URL провайдера с секретами.
            warn!(%attempt_id, "Не удалось подтвердить Google OAuth для desktop");
            let message = error
                .user_message()
                .unwrap_or("Не удалось войти через Google. Попробуй ещё раз.")
                .to_owned();
            if state
                .auth_store
                .fail_desktop_oauth_attempt(&attempt_id, message, Utc::now())
                .await
                .is_err()
            {
                warn!(%attempt_id, "Не удалось сохранить отказ desktop OAuth");
            }
            false
        }
    }
}

/// Атомарно фиксирует завершение перед изменением аккаунта или созданием сессии.
pub(super) async fn claim_handoff(
    state: &AppState,
    handoff: OAuthHandoff,
) -> Result<OAuthCompleteRequest, AuthError> {
    let flow = match handoff.kind.as_str() {
        "desktop_login" => "login",
        "desktop_link" => "link",
        _ => return Err(unavailable()),
    };
    // Личность читается до захвата: после него очистка по TTL уже не мешает завершению.
    let identity = state
        .auth_store
        .desktop_oauth_identity_for_handoff(&handoff.id)
        .await
        .map_err(AuthError::Internal)?
        .ok_or_else(unavailable)?;
    if !state
        .auth_store
        .consume_oauth_handoff(&handoff.id, Utc::now())
        .await
        .map_err(AuthError::Internal)?
    {
        warn!(handoff_id = %handoff.id, "Desktop OAuth уже завершён, отменён или истёк");
        return Err(unavailable());
    }
    let handoff_code = create_google_handoff(
        state,
        flow,
        handoff.user_id,
        &GoogleIdentity {
            subject: identity.subject,
            email: identity.email,
            display_name: identity.display_name,
        },
        Utc::now(),
    )
    .await?;
    info!(handoff_id = %handoff.id, flow, "Desktop атомарно подтвердил завершение Google OAuth");
    Ok(OAuthCompleteRequest { handoff_code })
}

fn unavailable() -> AuthError {
    AuthError::Unauthorized(
        "Вход через Google истёк, отменён или уже завершён. Начни вход ещё раз.".to_owned(),
    )
}
