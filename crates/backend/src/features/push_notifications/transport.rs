//! HTTP-адаптер регистрации push-установок.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use cheenhub_contracts::rest::{ApiError, UpsertPushInstallationRequest};

use crate::features::push_notifications::application;
use crate::features::push_notifications::error::PushError;
use crate::state::AppState;

/// Регистрирует или обновляет push-установку текущей auth-сессии.
pub(crate) async fn upsert_installation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(installation_id): Path<String>,
    Json(request): Json<UpsertPushInstallationRequest>,
) -> Result<StatusCode, PushError> {
    let token = bearer_token(&headers)?;
    application::upsert_installation(&state, token, installation_id, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Удаляет push-установку текущей auth-сессии.
pub(crate) async fn delete_installation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(installation_id): Path<String>,
) -> Result<StatusCode, PushError> {
    let token = bearer_token(&headers)?;
    application::delete_installation(&state, token, installation_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

impl IntoResponse for PushError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, "unauthorized", message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, "not_found", message),
            Self::Unavailable(message) => {
                (StatusCode::SERVICE_UNAVAILABLE, "push_unavailable", message)
            }
            Self::Internal(error) => {
                tracing::error!(%error, "push notifications request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Внутренняя ошибка сервера.".to_owned(),
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

fn bearer_token(headers: &HeaderMap) -> Result<&str, PushError> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| PushError::Unauthorized("Требуется авторизация.".to_owned()))?;
    value
        .strip_prefix("Bearer ")
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| PushError::Unauthorized("Требуется авторизация.".to_owned()))
}
