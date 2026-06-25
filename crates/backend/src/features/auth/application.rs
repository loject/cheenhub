//! Потоки приложения аутентификации.

use cheenhub_contracts::rest::{
    AuthResponse, AuthUser, ChangeCurrentUserPasswordRequest, LoginRequest, LogoutRequest,
    PasswordResetConfirmRequest, PasswordResetRequest, RefreshRequest, RegisterRequest,
    UpdateCurrentUserRequest,
};
use chrono::{Duration, Utc};

use crate::features::auth::domain::UserAccount;
use crate::features::auth::email::{EmailError, PasswordChangedEmail, PasswordResetEmail};
use crate::features::auth::error::AuthError;
use crate::features::auth::infrastructure::{
    InsertUserError, UpdateUserNicknameError, UserConflict,
};
use crate::features::auth::security::{jwt, password, refresh_token};
use crate::features::auth::validation;
use crate::state::AppState;
use uuid::Uuid;

mod avatar;
mod google;
mod oauth;
mod sessions;

const NICKNAME_CHANGE_COOLDOWN_DAYS: i64 = 7;
#[cfg(test)]
mod tests;

pub(crate) use avatar::update_current_user_avatar;
pub(crate) use oauth::{
    complete_google_oauth, google_oauth_callback_url, linked_accounts, register_with_google_oauth,
    start_google_oauth, unlink_google,
};
pub(crate) use sessions::{
    active_sessions, revoke_current_user_session, revoke_current_user_sessions,
};

/// Регистрирует пользователя и создает аутентифицированную сессию.
#[cfg(test)]
pub(crate) async fn register(
    state: &AppState,
    request: RegisterRequest,
) -> Result<AuthResponse, AuthError> {
    register_with_user_agent(state, request, None).await
}

/// Регистрирует пользователя и записывает метаданные User-Agent запроса, если они присутствуют.
pub(crate) async fn register_with_user_agent(
    state: &AppState,
    request: RegisterRequest,
    user_agent: Option<String>,
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
            Some(password_hash),
            now,
        )
        .await
        .map_err(map_insert_user_error)?;

    create_auth_response(state, &user, user_agent.as_deref()).await
}

/// Вход пользователя и создание аутентифицированной сессии.
#[cfg(test)]
pub(crate) async fn login(
    state: &AppState,
    request: LoginRequest,
) -> Result<AuthResponse, AuthError> {
    login_with_user_agent(state, request, None).await
}

/// Вход пользователя и запись метаданных User-Agent запроса, если они присутствуют.
pub(crate) async fn login_with_user_agent(
    state: &AppState,
    request: LoginRequest,
    user_agent: Option<String>,
) -> Result<AuthResponse, AuthError> {
    let valid = validation::login(request.email, request.password)
        .map_err(|message| AuthError::BadRequest(message.to_owned()))?;
    let Some(user) = state
        .auth_store
        .find_user_by_email(&valid.email_normalized)
        .await
        .map_err(AuthError::Internal)?
    else {
        // Выравниваем время ответа, чтобы нельзя было перечислять аккаунты по таймингу.
        password::verify_dummy_password();
        return Err(invalid_credentials());
    };

    let Some(password_hash) = &user.password_hash else {
        // Аккаунт без локального пароля (создан через внешний вход). Возвращаем
        // тот же обобщенный ответ и выполняем такую же работу, чтобы не раскрывать
        // ни факт существования аккаунта, ни способ его регистрации.
        password::verify_dummy_password();
        return Err(invalid_credentials());
    };

    if !password::verify_password(&valid.password, password_hash) {
        return Err(invalid_credentials());
    }

    create_auth_response(state, &user, user_agent.as_deref()).await
}

/// Отправляет письмо сброса пароля, если учетная запись существует.
pub(crate) async fn request_password_reset(
    state: &AppState,
    request: PasswordResetRequest,
) -> Result<(), AuthError> {
    let valid = validation::password_reset_request(request.email)
        .map_err(|message| AuthError::BadRequest(message.to_owned()))?;
    let now = Utc::now();
    let Some(user) = state
        .auth_store
        .find_user_by_email(&valid.email_normalized)
        .await
        .map_err(AuthError::Internal)?
    else {
        tracing::info!("accepted password reset request for unknown account");
        return Ok(());
    };

    let reset_token = refresh_token::generate();
    let reset_token_hash = refresh_token::hash(&reset_token);
    let expires_at = now + Duration::minutes(state.password_reset_token_lifetime_minutes);
    state
        .auth_store
        .insert_password_reset_token(&user.id, reset_token_hash, now, expires_at)
        .await
        .map_err(AuthError::Internal)?;

    let reset_url = format!(
        "{}/reset-password?token={}",
        state.cheenhub_client_base_url.trim_end_matches('/'),
        reset_token
    );
    tracing::info!(user_id = %user.id, "sending password reset email");
    state
        .auth_mailer
        .send_password_reset(PasswordResetEmail {
            to: user.email,
            reset_url,
        })
        .await
        .map_err(map_email_error)?;

    Ok(())
}

/// Подтверждает сброс пароля с использованием токена сброса пароля и устанавливает новый пароль.
pub(crate) async fn confirm_password_reset(
    state: &AppState,
    request: PasswordResetConfirmRequest,
) -> Result<(), AuthError> {
    let valid = validation::password_reset_confirm(request.token, request.new_password)
        .map_err(|message| AuthError::BadRequest(message.to_owned()))?;
    let now = Utc::now();
    let token_hash = refresh_token::hash(&valid.token);
    let Some(reset_token) = state
        .auth_store
        .consume_password_reset_token(&token_hash, now)
        .await
        .map_err(AuthError::Internal)?
    else {
        tracing::warn!("rejected invalid password reset token");
        return Err(AuthError::Unauthorized(
            "Ссылка для сброса пароля истекла или уже использована.".to_owned(),
        ));
    };
    tracing::info!(
        reset_token_id = %reset_token.id,
        user_id = %reset_token.user_id,
        "consumed password reset token"
    );

    let password_hash = password::hash_password(&valid.new_password)?;
    state
        .auth_store
        .update_user_password_hash(&reset_token.user_id, password_hash, now)
        .await
        .map_err(AuthError::Internal)?;
    state
        .auth_store
        .revoke_user_sessions(&reset_token.user_id, now)
        .await
        .map_err(AuthError::Internal)?;
    tracing::info!(user_id = %reset_token.user_id, "changed password through reset flow");

    Ok(())
}

/// Обновляет refresh-токен и записывает метаданные User-Agent запроса, если они присутствуют.
pub(crate) async fn refresh_with_user_agent(
    state: &AppState,
    request: RefreshRequest,
    user_agent: Option<String>,
) -> Result<AuthResponse, AuthError> {
    let old_hash = refresh_token::hash(&request.refresh_token);
    let now = Utc::now();
    let Some(refresh_session) = state
        .auth_store
        .find_active_refresh(&old_hash, now)
        .await
        .map_err(AuthError::Internal)?
    else {
        // Возможна кража токена: если предъявлен уже ротированный/отозванный токен,
        // отзываем всю сессию целиком, а не просто отвечаем отказом.
        if state
            .auth_store
            .revoke_session_on_refresh_reuse(&old_hash, now)
            .await
            .map_err(AuthError::Internal)?
        {
            tracing::warn!("detected refresh token reuse; revoked the entire session");
        }
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
            user_agent.as_deref(),
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
        user: auth_user(state, &refresh_session.user),
    })
}

/// Аннулирует текущую сессию refresh-токена.
pub(crate) async fn logout(state: &AppState, request: LogoutRequest) -> Result<(), AuthError> {
    let token_hash = refresh_token::hash(&request.refresh_token);
    state
        .auth_store
        .revoke_refresh_session(&token_hash, Utc::now())
        .await
        .map_err(AuthError::Internal)?;

    Ok(())
}

/// Возвращает пользователя для валидного access JWT.
pub(crate) async fn me(state: &AppState, access_token: &str) -> Result<AuthUser, AuthError> {
    let (user, _) = require_current_user(state, access_token).await?;

    Ok(auth_user(state, &user))
}

/// Обновляет профиль текущего пользователя.
pub(crate) async fn update_current_user(
    state: &AppState,
    access_token: &str,
    request: UpdateCurrentUserRequest,
) -> Result<AuthUser, AuthError> {
    let (user, session_id) = require_current_user(state, access_token).await?;
    let valid = validation::current_user_update(request.nickname)
        .map_err(|message| AuthError::BadRequest(message.to_owned()))?;
    if valid.nickname == user.nickname {
        return Ok(auth_user(state, &user));
    }

    let now = Utc::now();
    let cooldown = Duration::days(NICKNAME_CHANGE_COOLDOWN_DAYS);

    tracing::info!(user_id = %user.id, "updating user nickname");
    let updated_user = state
        .auth_store
        .update_user_nickname(&user.id, &session_id, valid.nickname, now, cooldown)
        .await
        .map_err(map_update_user_nickname_error)?
        .ok_or_else(expired_session)?;

    crate::features::voice_chat::application::update_user_nickname(
        state,
        &updated_user.id,
        updated_user.nickname.clone(),
    )
    .await;
    tracing::info!(user_id = %updated_user.id, "updated user nickname");

    Ok(auth_user(state, &updated_user))
}

/// Меняет пароль текущего пользователя.
pub(crate) async fn change_current_user_password(
    state: &AppState,
    access_token: &str,
    request: ChangeCurrentUserPasswordRequest,
) -> Result<(), AuthError> {
    let (user, session_id) = require_current_user(state, access_token).await?;
    let valid = validation::password_change(
        request.current_password,
        request.new_password,
        request.new_password_confirmation,
    )
    .map_err(|message| AuthError::BadRequest(message.to_owned()))?;

    if let Some(password_hash) = &user.password_hash
        && !password::verify_password(&valid.current_password, password_hash)
    {
        tracing::warn!(user_id = %user.id, "rejected password change with invalid current password");
        return Err(AuthError::Unauthorized(
            "Текущий пароль указан неверно.".to_owned(),
        ));
    }

    let now = Utc::now();
    let next_password_hash = password::hash_password(&valid.new_password)?;
    tracing::info!(user_id = %user.id, session_id = %session_id, "changing current user password");
    state
        .auth_store
        .change_user_password(&user.id, &session_id, next_password_hash, now)
        .await
        .map_err(AuthError::Internal)?;
    tracing::info!(user_id = %user.id, session_id = %session_id, "changed current user password");

    match state
        .auth_mailer
        .send_password_changed(PasswordChangedEmail {
            to: user.email.clone(),
        })
        .await
    {
        Ok(()) => tracing::info!(user_id = %user.id, "sent password change notification email"),
        Err(error) => {
            tracing::warn!(user_id = %user.id, ?error, "failed to send password change notification email")
        }
    }

    Ok(())
}

pub(super) async fn create_auth_response(
    state: &AppState,
    user: &UserAccount,
    user_agent: Option<&str>,
) -> Result<AuthResponse, AuthError> {
    let now = Utc::now();
    let refresh = refresh_token::generate();
    let refresh_hash = refresh_token::hash(&refresh);
    let expires_at = now + Duration::days(state.refresh_token_lifetime_days);
    let session_id = state
        .auth_store
        .create_session(&user.id, refresh_hash, user_agent, now, expires_at)
        .await
        .map_err(AuthError::Internal)?;
    tracing::info!(
        user_id = %user.id,
        %session_id,
        user_agent_present = user_agent.is_some(),
        "created auth session"
    );

    Ok(AuthResponse {
        access_token: jwt::sign_access_token(
            &state.auth_keys.signing_key,
            &state.auth_keys.key_id,
            state.access_token_lifetime_minutes,
            user,
            &session_id,
        )?,
        refresh_token: refresh,
        user: auth_user(state, user),
    })
}

pub(super) async fn require_current_user(
    state: &AppState,
    access_token: &str,
) -> Result<(UserAccount, Uuid), AuthError> {
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

    Ok((user, session_id))
}

pub(crate) fn auth_user(state: &AppState, user: &UserAccount) -> AuthUser {
    AuthUser {
        id: user.id.to_string(),
        nickname: user.nickname.clone(),
        email: user.email.clone(),
        registered_at: user.registered_at.to_rfc3339(),
        has_password: user.password_hash.is_some(),
        avatar_url: user
            .avatar_image_id
            .map(|image_id| crate::features::images::application::avatar_url(state, &image_id)),
    }
}

pub(super) fn map_insert_user_error(error: InsertUserError) -> AuthError {
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

fn map_update_user_nickname_error(error: UpdateUserNicknameError) -> AuthError {
    match error {
        UpdateUserNicknameError::Conflict(UserConflict::Nickname) => {
            AuthError::Conflict("Этот никнейм уже занят.".to_owned())
        }
        UpdateUserNicknameError::Conflict(UserConflict::Email) => {
            AuthError::Conflict("Этот email уже используется.".to_owned())
        }
        UpdateUserNicknameError::Cooldown { next_allowed_at } => AuthError::RateLimited(format!(
            "Никнейм можно изменить раз в 7 дней. Следующая смена будет доступна {}.",
            next_allowed_at.format("%d.%m.%Y %H:%M UTC")
        )),
        UpdateUserNicknameError::Database(error) => AuthError::Internal(error.into()),
        UpdateUserNicknameError::Storage(error) => AuthError::Internal(error),
    }
}

fn invalid_credentials() -> AuthError {
    AuthError::Unauthorized("Email или пароль указаны неверно.".to_owned())
}

pub(super) fn expired_session() -> AuthError {
    AuthError::Unauthorized("Сессия истекла. Войди снова.".to_owned())
}

fn map_email_error(error: EmailError) -> AuthError {
    match error {
        EmailError::Misconfigured { missing } => AuthError::Misconfigured {
            feature: "password_reset_email",
            missing,
            message: "Сброс пароля по email пока не настроен.".to_owned(),
        },
        EmailError::Internal(error) => {
            tracing::warn!(%error, "failed to send password reset email");
            AuthError::Internal(error)
        }
    }
}
