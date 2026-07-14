//! Потоки приложения для auth-сессий текущего пользователя.

use cheenhub_contracts::rest::{
    ActiveSession, ActiveSessionsResponse, SessionClientInfo, SessionDeviceKind,
};
use chrono::Utc;
use uuid::Uuid;

use super::require_current_user;
use crate::features::auth::domain::UserSession;
use crate::features::auth::error::AuthError;
use crate::features::auth::security::user_agent;
use crate::state::AppState;

/// Возвращает активные auth-сессии текущего пользователя.
#[cfg(test)]
pub(crate) async fn active_sessions(
    state: &AppState,
    access_token: &str,
) -> Result<ActiveSessionsResponse, AuthError> {
    active_sessions_with_user_agent(state, access_token, None).await
}

/// Возвращает активные auth-сессии и записывает User-Agent текущего запроса.
pub(crate) async fn active_sessions_with_user_agent(
    state: &AppState,
    access_token: &str,
    user_agent: Option<String>,
) -> Result<ActiveSessionsResponse, AuthError> {
    let (user, current_session_id) = require_current_user(state, access_token).await?;
    let now = Utc::now();
    if let Some(user_agent) = user_agent.as_deref() {
        state
            .auth_store
            .record_session_user_agent(&current_session_id, user_agent, now)
            .await
            .map_err(AuthError::Internal)?;
    }
    let sessions = state
        .auth_store
        .list_active_sessions(&user.id, now)
        .await
        .map_err(AuthError::Internal)?;
    tracing::info!(
        user_id = %user.id,
        active_session_count = sessions.len(),
        "listed active auth sessions"
    );

    Ok(ActiveSessionsResponse {
        sessions: sessions
            .into_iter()
            .map(|session| active_session_response(session, &current_session_id))
            .collect(),
    })
}

/// Отзывает одну активную auth-сессию, принадлежащую текущему пользователю.
pub(crate) async fn revoke_current_user_session(
    state: &AppState,
    access_token: &str,
    session_id: &str,
) -> Result<(), AuthError> {
    let target_session_id = Uuid::parse_str(session_id)
        .map_err(|_| AuthError::BadRequest("Некорректный идентификатор сессии.".to_owned()))?;
    let (user, current_session_id) = require_current_user(state, access_token).await?;
    let revoked = state
        .auth_store
        .revoke_user_session(&user.id, &target_session_id, Utc::now())
        .await
        .map_err(AuthError::Internal)?;

    if revoked {
        tracing::info!(
            user_id = %user.id,
            session_id = %target_session_id,
            current = target_session_id == current_session_id,
            "revoked auth session from security settings"
        );
    } else {
        tracing::warn!(
            user_id = %user.id,
            session_id = %target_session_id,
            "auth session revoke requested for missing or inactive session"
        );
    }

    Ok(())
}

/// Отзывает все активные auth-сессии, принадлежащие текущему пользователю.
pub(crate) async fn revoke_current_user_sessions(
    state: &AppState,
    access_token: &str,
) -> Result<(), AuthError> {
    let (user, current_session_id) = require_current_user(state, access_token).await?;
    state
        .auth_store
        .revoke_user_sessions(&user.id, Utc::now())
        .await
        .map_err(AuthError::Internal)?;
    tracing::info!(
        user_id = %user.id,
        current_session_id = %current_session_id,
        "revoked all auth sessions from security settings"
    );

    Ok(())
}

fn active_session_response(session: UserSession, current_session_id: &Uuid) -> ActiveSession {
    let parsed = user_agent::parse(session.user_agent.as_deref());

    ActiveSession {
        id: session.id.to_string(),
        client: SessionClientInfo {
            device_kind: match parsed.device_kind {
                user_agent::ParsedDeviceKind::Desktop => SessionDeviceKind::Desktop,
                user_agent::ParsedDeviceKind::Mobile => SessionDeviceKind::Mobile,
                user_agent::ParsedDeviceKind::Tablet => SessionDeviceKind::Tablet,
                user_agent::ParsedDeviceKind::Bot => SessionDeviceKind::Bot,
                user_agent::ParsedDeviceKind::Unknown => SessionDeviceKind::Unknown,
            },
            os_name: parsed.os_name,
            browser_name: parsed.browser_name,
        },
        user_agent: session.user_agent,
        created_at: session.created_at.to_rfc3339(),
        last_seen_at: session.last_seen_at.to_rfc3339(),
        expires_at: session.expires_at.to_rfc3339(),
        current: session.id == *current_session_id,
    }
}
