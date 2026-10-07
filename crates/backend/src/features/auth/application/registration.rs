//! Регистрация по email и паролю.

use cheenhub_contracts::rest::{AuthResponse, RegisterRequest};
use chrono::Utc;

use super::{create_auth_response, legal, map_insert_user_error};
use crate::features::auth::error::AuthError;
use crate::features::auth::security::password;
use crate::features::auth::validation;
use crate::state::AppState;

/// Регистрирует пользователя и создаёт аутентифицированную сессию.
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
    if !crate::features::host_settings::registration_settings::email_password_registration_enabled(
        state,
    )
    .await
    .map_err(AuthError::Internal)?
    {
        tracing::warn!("rejected email and password registration because host settings disable it");
        return Err(AuthError::BadRequest(
            "Регистрация по email и паролю сейчас недоступна.".to_owned(),
        ));
    }
    legal::validate_registration_acceptance(
        request.accepts_terms,
        request.accepts_personal_data,
        "password",
    )?;
    let valid = validation::register(request.nickname, request.email, request.password)
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
            legal::current_acceptance("password"),
            now,
        )
        .await
        .map_err(map_insert_user_error)?;
    legal::log_recorded(&user.id, "password");

    create_auth_response(state, &user, user_agent.as_deref()).await
}
