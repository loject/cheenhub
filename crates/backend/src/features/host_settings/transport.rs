//! REST-адаптер глобальных настроек хоста.

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use cheenhub_contracts::rest::{
    ApiError, GmailConnectionStartResponse, HostAccessResponse, HostEmailSettingsResponse,
    UpdateHostEmailSettingsRequest,
};
use serde::Deserialize;

use crate::state::AppState;

use super::application::{self, HostSettingsError};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/access", get(access))
        .route("/metrics", get(metrics))
        .route("/email", get(email_settings).patch(update_email_settings))
        .route("/email/gmail/connect", post(start_gmail_connection))
        .route("/email/gmail/callback", get(gmail_callback))
        .route("/email/gmail/disconnect", post(disconnect_gmail))
}

async fn metrics(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<cheenhub_contracts::rest::HostMetricsResponse>, HostSettingsError> {
    application::metrics(&state, bearer_token(&headers)?)
        .await
        .map(Json)
}

async fn access(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<HostAccessResponse>, HostSettingsError> {
    application::access(&state, bearer_token(&headers)?)
        .await
        .map(Json)
}

async fn email_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<HostEmailSettingsResponse>, HostSettingsError> {
    application::email_settings(&state, bearer_token(&headers)?)
        .await
        .map(Json)
}

async fn update_email_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UpdateHostEmailSettingsRequest>,
) -> Result<Json<HostEmailSettingsResponse>, HostSettingsError> {
    application::update_email_settings(&state, bearer_token(&headers)?, request)
        .await
        .map(Json)
}

async fn start_gmail_connection(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<GmailConnectionStartResponse>, HostSettingsError> {
    application::start_gmail_connection(&state, bearer_token(&headers)?)
        .await
        .map(Json)
}

async fn gmail_callback(
    State(state): State<AppState>,
    Query(query): Query<GmailCallbackQuery>,
) -> Redirect {
    Redirect::to(
        &application::complete_gmail_connection(&state, query.code, query.state, query.error).await,
    )
}

async fn disconnect_gmail(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<HostEmailSettingsResponse>, HostSettingsError> {
    application::disconnect_gmail(&state, bearer_token(&headers)?)
        .await
        .map(Json)
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, HostSettingsError> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
        .ok_or_else(|| HostSettingsError::Unauthorized("Войди, чтобы продолжить.".to_owned()))
}

impl IntoResponse for HostSettingsError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Unauthorized(message) => (StatusCode::UNAUTHORIZED, "unauthorized", message),
            Self::Forbidden(message) => (StatusCode::FORBIDDEN, "forbidden", message),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::Misconfigured(message) => {
                (StatusCode::SERVICE_UNAVAILABLE, "misconfigured", message)
            }
            Self::Internal(error) => {
                tracing::error!(%error, "host settings request failed");
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

#[derive(Deserialize)]
struct GmailCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}
