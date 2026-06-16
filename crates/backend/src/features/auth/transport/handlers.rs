//! HTTP обработчики аутентификации.

use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use cheenhub_contracts::rest::{
    ActiveSessionsResponse, ApiError, AuthResponse, AuthUser, ChangeCurrentUserPasswordRequest,
    LinkedAccountsResponse, LoginRequest, LogoutRequest, OAuthCompleteRequest,
    OAuthCompleteResponse, OAuthRegistrationRequest, OAuthStartRequest, OAuthStartResponse,
    PasswordResetConfirmRequest, PasswordResetRequest, RefreshRequest, RegisterRequest,
    UpdateCurrentUserRequest,
};
use serde::Deserialize;

use crate::features::auth::application;
use crate::features::auth::error::AuthError;
use crate::state::AppState;

/// Регистрирует новую учетную запись email/пароль.
pub(crate) async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    application::register_with_user_agent(&state, request, request_user_agent(&headers))
        .await
        .map(Json)
}

/// Вход по email/паролю.
pub(crate) async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    application::login_with_user_agent(&state, request, request_user_agent(&headers))
        .await
        .map(Json)
}

/// Отправляет письмо сброса пароля, если учетная запись существует.
pub(crate) async fn request_password_reset(
    State(state): State<AppState>,
    Json(request): Json<PasswordResetRequest>,
) -> Result<StatusCode, AuthError> {
    application::request_password_reset(&state, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Устанавливает новый пароль с использованием токена сброса пароля.
pub(crate) async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(request): Json<PasswordResetConfirmRequest>,
) -> Result<StatusCode, AuthError> {
    application::confirm_password_reset(&state, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Обновляет refresh-токен и возвращает новую пару токенов.
pub(crate) async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    application::refresh_with_user_agent(&state, request, request_user_agent(&headers))
        .await
        .map(Json)
}

/// Аннулирует текущую сессию refresh-токена.
pub(crate) async fn logout(
    State(state): State<AppState>,
    Json(request): Json<LogoutRequest>,
) -> Result<StatusCode, AuthError> {
    application::logout(&state, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Список активных сессий текущего пользователя.
pub(crate) async fn active_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ActiveSessionsResponse>, AuthError> {
    let token = bearer_token(&headers)?;
    application::active_sessions(&state, token).await.map(Json)
}

/// Аннулирует одну активную сессию, принадлежащую текущему пользователю.
pub(crate) async fn revoke_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<StatusCode, AuthError> {
    let token = bearer_token(&headers)?;
    application::revoke_current_user_session(&state, token, &session_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Аннулирует все активные сессии текущего пользователя.
pub(crate) async fn revoke_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<StatusCode, AuthError> {
    let token = bearer_token(&headers)?;
    application::revoke_current_user_sessions(&state, token).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Возвращает текущего пользователя для валидного Access JWT.
pub(crate) async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthUser>, AuthError> {
    let token = bearer_token(&headers)?;
    application::me(&state, token).await.map(Json)
}

/// Обновляет профиль текущего пользователя.
pub(crate) async fn update_current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UpdateCurrentUserRequest>,
) -> Result<Json<AuthUser>, AuthError> {
    let token = bearer_token(&headers)?;
    application::update_current_user(&state, token, request)
        .await
        .map(Json)
}

/// Меняет пароль текущего пользователя.
pub(crate) async fn change_current_user_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChangeCurrentUserPasswordRequest>,
) -> Result<StatusCode, AuthError> {
    let token = bearer_token(&headers)?;
    application::change_current_user_password(&state, token, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Обновляет аватар текущего пользователя.
pub(crate) async fn update_current_user_avatar(
    State(state): State<AppState>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<Json<AuthUser>, AuthError> {
    let token = bearer_token(&headers)?;
    application::update_current_user_avatar(&state, token, bytes)
        .await
        .map(Json)
}

/// Запускает процесс входа Google OAuth или привязки.
pub(crate) async fn start_google_oauth(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<OAuthStartRequest>,
) -> Result<Json<OAuthStartResponse>, AuthError> {
    let token = optional_bearer_token(&headers);
    application::start_google_oauth(&state, token, request)
        .await
        .map(Json)
}

/// Обрабатывает callback провайдера Google OAuth.
pub(crate) async fn google_oauth_callback(
    State(state): State<AppState>,
    Query(query): Query<GoogleOAuthCallbackQuery>,
) -> Redirect {
    let url =
        application::google_oauth_callback_url(&state, query.code, query.state, query.error).await;
    Redirect::to(&url)
}

/// Завершает процесс Google OAuth frontend handoff.
pub(crate) async fn complete_google_oauth(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<OAuthCompleteRequest>,
) -> Result<Json<OAuthCompleteResponse>, AuthError> {
    application::complete_google_oauth(&state, request, request_user_agent(&headers))
        .await
        .map(Json)
}

/// Завершает регистрацию для подтвержденной личности Google OAuth.
pub(crate) async fn register_with_google_oauth(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<OAuthRegistrationRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    application::register_with_google_oauth(&state, request, request_user_agent(&headers))
        .await
        .map(Json)
}

/// Lists external accounts linked to the current user.
pub(crate) async fn linked_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<LinkedAccountsResponse>, AuthError> {
    let token = bearer_token(&headers)?;
    application::linked_accounts(&state, token).await.map(Json)
}

/// Unlinks Google from the current user.
pub(crate) async fn unlink_google(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<LinkedAccountsResponse>, AuthError> {
    let token = bearer_token(&headers)?;
    application::unlink_google(&state, token).await.map(Json)
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, "unauthorized", message),
            Self::Conflict(message) => (StatusCode::CONFLICT, "conflict", message),
            Self::RateLimited(message) => (StatusCode::TOO_MANY_REQUESTS, "rate_limited", message),
            Self::Misconfigured {
                feature,
                missing,
                message,
            } => {
                tracing::warn!(
                    feature,
                    missing_env = ?missing,
                    "authentication feature is not configured; set the listed environment variables in .env and restart the backend"
                );
                (StatusCode::SERVICE_UNAVAILABLE, "misconfigured", message)
            }
            Self::Internal(error) => {
                tracing::error!(%error, "authentication request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Не удалось выполнить запрос. Попробуй еще раз.".to_owned(),
                )
            }
        };

        (
            status,
            Json(ApiError {
                code: code.to_owned(),
                message,
            }),
        )
            .into_response()
    }
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, AuthError> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AuthError::Unauthorized("Войди, чтобы продолжить.".to_owned()))?;

    value
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
        .ok_or_else(|| AuthError::Unauthorized("Войди, чтобы продолжить.".to_owned()))
}

fn optional_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
}

fn request_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

#[derive(Debug, Deserialize)]
pub(crate) struct GoogleOAuthCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}
