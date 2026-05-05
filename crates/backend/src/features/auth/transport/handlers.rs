//! Authentication HTTP handlers.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use cheenhub_contracts::rest::{
    ApiError, AuthResponse, AuthUser, LoginRequest, LogoutRequest, RefreshRequest, RegisterRequest,
};

use crate::features::auth::application;
use crate::features::auth::error::AuthError;
use crate::http::AppState;

/// Registers a new email/password account.
pub(crate) async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    application::register(&state, request).await.map(Json)
}

/// Logs in with email/password.
pub(crate) async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    application::login(&state, request).await.map(Json)
}

/// Rotates a refresh token and returns a new token pair.
pub(crate) async fn refresh(
    State(state): State<AppState>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    application::refresh(&state, request).await.map(Json)
}

/// Invalidates the current refresh session.
pub(crate) async fn logout(
    State(state): State<AppState>,
    Json(request): Json<LogoutRequest>,
) -> Result<StatusCode, AuthError> {
    application::logout(&state, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Returns the current user for a valid access JWT.
pub(crate) async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AuthUser>, AuthError> {
    let token = bearer_token(&headers)?;
    application::me(&state, token).await.map(Json)
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, "unauthorized", message),
            Self::Conflict(message) => (StatusCode::CONFLICT, "conflict", message),
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
