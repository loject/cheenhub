//! Server HTTP handlers.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use cheenhub_contracts::rest::{
    ApiError, CreateServerRequest, CreateServerResponse, ListServersResponse,
};

use crate::features::servers::application;
use crate::features::servers::error::ServerError;
use crate::http::AppState;

/// Creates a server owned by the current user.
pub(crate) async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateServerRequest>,
) -> Result<Json<CreateServerResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::create(&state, token, request).await.map(Json)
}

/// Lists servers owned by the current user.
pub(crate) async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListServersResponse>, ServerError> {
    let token = bearer_token(&headers)?;
    application::list(&state, token).await.map(Json)
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, "unauthorized", message),
            Self::Internal(error) => {
                tracing::error!(%error, "server request failed");
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

// TODO: метод встречается в нескольких местах
fn bearer_token(headers: &HeaderMap) -> Result<&str, ServerError> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ServerError::Unauthorized("Войди, чтобы продолжить.".to_owned()))?;

    value
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
        .ok_or_else(|| ServerError::Unauthorized("Войди, чтобы продолжить.".to_owned()))
}
