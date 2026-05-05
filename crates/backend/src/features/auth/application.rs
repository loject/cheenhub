//! Authentication application flows.

use cheenhub_contracts::rest::{
    AuthResponse, AuthUser, LoginRequest, LogoutRequest, RefreshRequest, RegisterRequest,
};
use chrono::{Duration, Utc};

use crate::features::auth::domain::UserAccount;
use crate::features::auth::error::AuthError;
use crate::features::auth::infrastructure::{InsertUserError, UserConflict};
use crate::features::auth::security::{jwt, password, refresh_token};
use crate::features::auth::validation;
use crate::http::AppState;
use uuid::Uuid;

/// Registers a user and creates an authenticated session.
pub(crate) async fn register(
    state: &AppState,
    request: RegisterRequest,
) -> Result<AuthResponse, AuthError> {
    let valid = validation::register(
        request.nickname,
        request.email,
        request.password,
        request.accepts_policies,
    )
    .map_err(|message| AuthError::BadRequest(message.to_owned()))?;
    let password_hash = password::hash_password(&valid.password)?;
    let now = Utc::now();
    let user = state
        .auth_store
        .insert_user(
            valid.nickname,
            valid.email,
            valid.email_normalized,
            password_hash,
            now,
        )
        .await
        .map_err(map_insert_user_error)?;

    create_auth_response(state, &user).await
}

/// Logs a user in and creates an authenticated session.
pub(crate) async fn login(
    state: &AppState,
    request: LoginRequest,
) -> Result<AuthResponse, AuthError> {
    let valid = validation::login(request.email, request.password)
        .map_err(|message| AuthError::BadRequest(message.to_owned()))?;
    let Some(user) = state
        .auth_store
        .find_user_by_email(&valid.email_normalized)
        .await
        .map_err(AuthError::Internal)?
    else {
        return Err(invalid_credentials());
    };

    if !password::verify_password(&valid.password, &user.password_hash) {
        return Err(invalid_credentials());
    }

    create_auth_response(state, &user).await
}

/// Rotates a refresh token and returns a fresh token pair.
pub(crate) async fn refresh(
    state: &AppState,
    request: RefreshRequest,
) -> Result<AuthResponse, AuthError> {
    let old_hash = refresh_token::hash(&request.refresh_token);
    let now = Utc::now();
    let Some(refresh_session) = state
        .auth_store
        .find_active_refresh(&old_hash, now)
        .await
        .map_err(AuthError::Internal)?
    else {
        return Err(expired_session());
    };

    let next_refresh = refresh_token::generate();
    let next_hash = refresh_token::hash(&next_refresh);
    let session_expires_at = now + Duration::days(state.refresh_token_lifetime_days);

    state
        .auth_store
        .rotate_refresh(
            &refresh_session.refresh_token_id,
            &refresh_session.session_id,
            next_hash,
            now,
            session_expires_at,
        )
        .await
        .map_err(AuthError::Internal)?;

    Ok(AuthResponse {
        access_token: jwt::sign_access_token(
            &state.auth_keys.signing_key,
            &state.auth_keys.key_id,
            state.access_token_lifetime_minutes,
            &refresh_session.user,
            &refresh_session.session_id,
        )?,
        refresh_token: next_refresh,
        user: auth_user(&refresh_session.user),
    })
}

/// Revokes a refresh session.
pub(crate) async fn logout(state: &AppState, request: LogoutRequest) -> Result<(), AuthError> {
    let token_hash = refresh_token::hash(&request.refresh_token);
    state
        .auth_store
        .revoke_refresh_session(&token_hash, Utc::now())
        .await
        .map_err(AuthError::Internal)?;

    Ok(())
}

/// Returns the user for a valid access JWT.
pub(crate) async fn me(state: &AppState, access_token: &str) -> Result<AuthUser, AuthError> {
    let claims = jwt::verify_access_token(&state.auth_keys, access_token)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| expired_session())?;
    let session_id = Uuid::parse_str(&claims.session_id).map_err(|_| expired_session())?;
    if !state
        .auth_store
        .session_is_active(&session_id, Utc::now())
        .await
        .map_err(AuthError::Internal)?
    {
        return Err(expired_session());
    }
    let Some(user) = state
        .auth_store
        .find_user_by_id(&user_id)
        .await
        .map_err(AuthError::Internal)?
    else {
        return Err(expired_session());
    };

    Ok(auth_user(&user))
}

async fn create_auth_response(
    state: &AppState,
    user: &UserAccount,
) -> Result<AuthResponse, AuthError> {
    let now = Utc::now();
    let refresh = refresh_token::generate();
    let refresh_hash = refresh_token::hash(&refresh);
    let expires_at = now + Duration::days(state.refresh_token_lifetime_days);
    let session_id = state
        .auth_store
        .create_session(&user.id, refresh_hash, now, expires_at)
        .await
        .map_err(AuthError::Internal)?;

    Ok(AuthResponse {
        access_token: jwt::sign_access_token(
            &state.auth_keys.signing_key,
            &state.auth_keys.key_id,
            state.access_token_lifetime_minutes,
            user,
            &session_id,
        )?,
        refresh_token: refresh,
        user: auth_user(user),
    })
}

fn auth_user(user: &UserAccount) -> AuthUser {
    AuthUser {
        id: user.id.to_string(),
        nickname: user.nickname.clone(),
        email: user.email.clone(),
        registered_at: user.registered_at.to_rfc3339(),
    }
}

fn map_insert_user_error(error: InsertUserError) -> AuthError {
    match error {
        InsertUserError::Conflict(UserConflict::Nickname) => {
            AuthError::Conflict("Этот никнейм уже занят.".to_owned())
        }
        InsertUserError::Conflict(UserConflict::Email) => {
            AuthError::Conflict("Этот email уже используется.".to_owned())
        }
        InsertUserError::Database(error) => AuthError::Internal(error.into()),
        InsertUserError::Storage(error) => AuthError::Internal(error),
    }
}

fn invalid_credentials() -> AuthError {
    AuthError::Unauthorized("Email или пароль указаны неверно.".to_owned())
}

fn expired_session() -> AuthError {
    AuthError::Unauthorized("Сессия истекла. Войди снова.".to_owned())
}
