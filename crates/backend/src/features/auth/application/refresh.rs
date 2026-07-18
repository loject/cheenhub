//! Ротация refresh-токенов и классификация отказов сессии.

use cheenhub_contracts::rest::{AuthResponse, RefreshRequest};
use chrono::{Duration, Utc};

use crate::features::auth::error::{AuthError, RefreshRejection};
use crate::features::auth::infrastructure::{RefreshReuseOutcome, RotateRefreshOutcome};
use crate::features::auth::security::{jwt, refresh_token};
use crate::state::AppState;

/// Атомарно ротирует refresh-токен и записывает User-Agent запроса.
pub(super) async fn execute(
    state: &AppState,
    request: RefreshRequest,
    user_agent: Option<String>,
) -> Result<AuthResponse, AuthError> {
    const CONCURRENT_ROTATION_GRACE_SECONDS: i64 = 5;

    let old_hash = refresh_token::hash(&request.refresh_token);
    let now = Utc::now();
    let concurrent_rotation_after = now - Duration::seconds(CONCURRENT_ROTATION_GRACE_SECONDS);
    let Some(refresh_session) = state
        .auth_store
        .find_active_refresh(&old_hash, now)
        .await
        .map_err(AuthError::Internal)?
    else {
        return Err(
            classify_inactive_token(state, &old_hash, now, concurrent_rotation_after).await?,
        );
    };

    let next_refresh = refresh_token::generate();
    let rotation = state
        .auth_store
        .rotate_refresh(
            &refresh_session.refresh_token_id,
            &refresh_session.session_id,
            refresh_token::hash(&next_refresh),
            user_agent.as_deref(),
            now,
            now + Duration::days(state.refresh_token_lifetime_days),
        )
        .await
        .map_err(AuthError::Internal)?;
    if rotation == RotateRefreshOutcome::AlreadyConsumed {
        return Err(classify_lost_rotation(
            state,
            &old_hash,
            &refresh_session.session_id,
            &refresh_session.user.id,
            now,
            concurrent_rotation_after,
        )
        .await?);
    }

    tracing::info!(
        session_id = %refresh_session.session_id,
        user_id = %refresh_session.user.id,
        "rotated auth refresh token"
    );
    Ok(AuthResponse {
        access_token: jwt::sign_access_token(
            &state.auth_keys.signing_key,
            &state.auth_keys.key_id,
            state.access_token_lifetime_minutes,
            &refresh_session.user,
            &refresh_session.session_id,
        )?,
        refresh_token: next_refresh,
        user: super::auth_user(state, &refresh_session.user),
    })
}

async fn classify_inactive_token(
    state: &AppState,
    token_hash: &str,
    now: chrono::DateTime<Utc>,
    concurrent_rotation_after: chrono::DateTime<Utc>,
) -> Result<AuthError, AuthError> {
    let outcome = state
        .auth_store
        .revoke_session_on_refresh_reuse(token_hash, now, concurrent_rotation_after)
        .await
        .map_err(AuthError::Internal)?;
    Ok(match outcome {
        RefreshReuseOutcome::ReusedAndRevoked => {
            tracing::warn!("detected refresh token reuse; revoked the entire session");
            reused_error()
        }
        RefreshReuseOutcome::ConcurrentRotation => {
            tracing::info!("deferred concurrent refresh request after recent token rotation");
            concurrent_error()
        }
        RefreshReuseOutcome::SessionRevoked => {
            tracing::info!("rejected refresh for an explicitly revoked auth session");
            revoked_error()
        }
        RefreshReuseOutcome::NotDetected => {
            tracing::warn!("rejected inactive or unknown refresh token");
            invalid_error()
        }
    })
}

async fn classify_lost_rotation(
    state: &AppState,
    token_hash: &str,
    session_id: &uuid::Uuid,
    user_id: &uuid::Uuid,
    now: chrono::DateTime<Utc>,
    concurrent_rotation_after: chrono::DateTime<Utc>,
) -> Result<AuthError, AuthError> {
    let outcome = state
        .auth_store
        .revoke_session_on_refresh_reuse(token_hash, now, concurrent_rotation_after)
        .await
        .map_err(AuthError::Internal)?;
    Ok(match outcome {
        RefreshReuseOutcome::ConcurrentRotation => {
            tracing::info!(%session_id, %user_id, "lost concurrent auth refresh rotation; preserving session during grace window");
            concurrent_error()
        }
        RefreshReuseOutcome::ReusedAndRevoked => {
            tracing::warn!(%session_id, %user_id, "lost auth refresh rotation outside grace window; revoked session");
            reused_error()
        }
        RefreshReuseOutcome::SessionRevoked => {
            tracing::info!(%session_id, %user_id, "auth refresh lost rotation because session was explicitly revoked");
            revoked_error()
        }
        RefreshReuseOutcome::NotDetected => {
            tracing::warn!(%session_id, %user_id, "lost auth refresh rotation without stored reuse evidence");
            invalid_error()
        }
    })
}

fn concurrent_error() -> AuthError {
    AuthError::RefreshRotationInProgress(
        "Сессия уже обновляется в другом запросе. Повтори попытку.".to_owned(),
    )
}

fn reused_error() -> AuthError {
    AuthError::RefreshRejected {
        reason: RefreshRejection::Reused,
        message: "Сессия завершена из-за повторного использования refresh-токена. Войди снова."
            .to_owned(),
    }
}

fn revoked_error() -> AuthError {
    AuthError::RefreshRejected {
        reason: RefreshRejection::SessionRevoked,
        message: "Сессия завершена на сервере. Войди снова.".to_owned(),
    }
}

fn invalid_error() -> AuthError {
    AuthError::RefreshRejected {
        reason: RefreshRejection::InvalidOrExpired,
        message: "Сессия истекла. Войди снова.".to_owned(),
    }
}
