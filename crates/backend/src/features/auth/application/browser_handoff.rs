//! Передача авторизованной веб-сессии в исходный desktop-клиент без браузерных токенов.

use cheenhub_contracts::rest::{
    AuthResponse, BrowserDesktopAuthDetailsResponse, BrowserDesktopAuthPollResponse,
    BrowserDesktopAuthRequest, BrowserDesktopAuthStartResponse,
};
use chrono::{Duration, Utc};
use tracing::{info, warn};
use uuid::Uuid;

use super::{create_auth_response, require_current_user};
use crate::features::auth::error::AuthError;
use crate::features::auth::security::refresh_token;
use crate::state::AppState;

const ATTEMPT_SECONDS: i64 = 300;
const POLL_SECONDS: u64 = 2;

/// Создаёт краткоживущую попытку, которую может завершить только владелец секрета.
pub(crate) async fn start_browser_handoff(
    state: &AppState,
) -> Result<BrowserDesktopAuthStartResponse, AuthError> {
    let now = Utc::now();
    let poll_secret = refresh_token::generate();
    let attempt_id = state
        .auth_store
        .insert_browser_handoff(
            refresh_token::hash(&poll_secret),
            now,
            now + Duration::seconds(ATTEMPT_SECONDS),
        )
        .await
        .map_err(AuthError::Internal)?;
    info!(%attempt_id, "created desktop browser sign-in attempt");
    Ok(BrowserDesktopAuthStartResponse {
        attempt_id,
        poll_secret,
        authorization_url: format!(
            "{}/auth/desktop?attempt={attempt_id}",
            state.cheenhub_client_base_url.trim_end_matches('/'),
        ),
        verification_code: verification_code(attempt_id),
        expires_in_seconds: ATTEMPT_SECONDS as u64,
        poll_interval_seconds: POLL_SECONDS,
    })
}

/// Возвращает только короткий проверочный код действующей попытки.
pub(crate) async fn browser_handoff_details(
    state: &AppState,
    attempt_id: Uuid,
) -> Result<BrowserDesktopAuthDetailsResponse, AuthError> {
    state
        .auth_store
        .find_browser_handoff(&attempt_id, Utc::now())
        .await
        .map_err(AuthError::Internal)?
        .ok_or_else(unavailable)?;
    Ok(BrowserDesktopAuthDetailsResponse {
        verification_code: verification_code(attempt_id),
    })
}

/// Привязывает к ожиданию пользователя действующей браузерной auth-сессии.
pub(crate) async fn approve_browser_handoff(
    state: &AppState,
    attempt_id: Uuid,
    access_token: &str,
) -> Result<(), AuthError> {
    let (user, _) = require_current_user(state, access_token).await?;
    let now = Utc::now();
    if state
        .auth_store
        .approve_browser_handoff(&attempt_id, &user.id, now)
        .await
        .map_err(AuthError::Internal)?
    {
        info!(%attempt_id, user_id = %user.id, "approved desktop browser sign-in");
        return Ok(());
    }
    let previous = state
        .auth_store
        .find_browser_handoff(&attempt_id, now)
        .await
        .map_err(AuthError::Internal)?;
    if previous
        .is_some_and(|handoff| handoff.kind == "browser_ready" && handoff.user_id == Some(user.id))
    {
        return Ok(());
    }
    warn!(%attempt_id, "rejected unavailable desktop browser approval");
    Err(unavailable())
}

/// Читает состояние только по секрету исходного приложения.
pub(crate) async fn poll_browser_handoff(
    state: &AppState,
    request: BrowserDesktopAuthRequest,
) -> Result<BrowserDesktopAuthPollResponse, AuthError> {
    let handoff = lookup(state, &request).await?;
    match handoff.kind.as_str() {
        "browser_pending" => Ok(BrowserDesktopAuthPollResponse::Pending),
        "browser_ready" if handoff.user_id.is_some() => Ok(BrowserDesktopAuthPollResponse::Ready),
        _ => Err(unavailable()),
    }
}

/// Отменяет неиспользованную попытку по секрету приложения.
pub(crate) async fn cancel_browser_handoff(
    state: &AppState,
    request: BrowserDesktopAuthRequest,
) -> Result<(), AuthError> {
    let handoff = lookup(state, &request).await?;
    let cancelled = state
        .auth_store
        .consume_oauth_handoff(&handoff.id, Utc::now())
        .await
        .map_err(AuthError::Internal)?;
    info!(attempt_id = %request.attempt_id, cancelled, "cancelled desktop browser sign-in");
    Ok(())
}

/// Атомарно забирает подтверждение и создаёт новую независимую auth-сессию.
pub(crate) async fn complete_browser_handoff(
    state: &AppState,
    request: BrowserDesktopAuthRequest,
    user_agent: Option<String>,
) -> Result<AuthResponse, AuthError> {
    let handoff = lookup(state, &request).await?;
    if handoff.kind != "browser_ready" {
        return Err(unavailable());
    }
    let user_id = handoff.user_id.ok_or_else(unavailable)?;
    if !state
        .auth_store
        .consume_oauth_handoff(&handoff.id, Utc::now())
        .await
        .map_err(AuthError::Internal)?
    {
        return Err(unavailable());
    }
    let user = state
        .auth_store
        .find_user_by_id(&user_id)
        .await
        .map_err(AuthError::Internal)?
        .ok_or_else(unavailable)?;
    let auth = create_auth_response(state, &user, user_agent.as_deref()).await?;
    info!(attempt_id = %request.attempt_id, %user_id, "completed desktop browser sign-in");
    Ok(auth)
}

async fn lookup(
    state: &AppState,
    request: &BrowserDesktopAuthRequest,
) -> Result<crate::features::auth::domain::OAuthHandoff, AuthError> {
    let handoff = state
        .auth_store
        .find_active_oauth_handoff(&refresh_token::hash(&request.poll_secret), Utc::now())
        .await
        .map_err(AuthError::Internal)?;
    handoff
        .filter(|value| {
            value.id == request.attempt_id
                && matches!(value.kind.as_str(), "browser_pending" | "browser_ready")
        })
        .ok_or_else(unavailable)
}

fn verification_code(id: Uuid) -> String {
    let value = id.as_simple().to_string();
    format!("{}-{}", &value[..4], &value[4..8]).to_uppercase()
}

fn unavailable() -> AuthError {
    AuthError::Unauthorized(
        "Подтверждение входа истекло, отменено или уже использовано. Попробуй ещё раз.".to_owned(),
    )
}
