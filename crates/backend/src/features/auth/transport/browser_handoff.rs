//! HTTP-адаптер передачи входа из браузера в исходное desktop-приложение.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use cheenhub_contracts::rest::BrowserDesktopAuthRequest;
use uuid::Uuid;

use crate::features::auth::{application, error::AuthError};
use crate::state::AppState;

/// Создаёт одноразовую попытку входа и ссылку на браузерное подтверждение.
pub(crate) async fn start(State(state): State<AppState>) -> Response {
    private_response(application::start_browser_handoff(&state).await.map(Json))
}

/// Возвращает проверочный код действующей попытки без секретов.
pub(crate) async fn details(
    State(state): State<AppState>,
    Path(attempt_id): Path<Uuid>,
) -> Response {
    private_response(
        application::browser_handoff_details(&state, attempt_id)
            .await
            .map(Json),
    )
}

/// Подтверждает вход от имени пользователя действующей веб-сессии.
pub(crate) async fn approve(
    State(state): State<AppState>,
    Path(attempt_id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    let result = bearer(&headers).ok_or_else(|| {
        AuthError::Unauthorized("Войди в браузере, чтобы подтвердить вход.".to_owned())
    });
    private_response(match result {
        Ok(token) => application::approve_browser_handoff(&state, attempt_id, token)
            .await
            .map(|()| StatusCode::NO_CONTENT),
        Err(error) => Err(error),
    })
}

/// Возвращает статус только исходному приложению, владеющему секретом.
pub(crate) async fn poll(
    State(state): State<AppState>,
    Json(request): Json<BrowserDesktopAuthRequest>,
) -> Response {
    private_response(
        application::poll_browser_handoff(&state, request)
            .await
            .map(Json),
    )
}

/// Отменяет попытку по секрету исходного приложения.
pub(crate) async fn cancel(
    State(state): State<AppState>,
    Json(request): Json<BrowserDesktopAuthRequest>,
) -> Response {
    private_response(
        application::cancel_browser_handoff(&state, request)
            .await
            .map(|()| StatusCode::NO_CONTENT),
    )
}

/// Однократно создаёт отдельную auth-сессию desktop-приложения.
pub(crate) async fn complete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<BrowserDesktopAuthRequest>,
) -> Response {
    let agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    private_response(
        application::complete_browser_handoff(&state, request, agent)
            .await
            .map(Json),
    )
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

fn private_response(response: Result<impl IntoResponse, AuthError>) -> Response {
    (
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::REFERRER_POLICY, "no-referrer"),
        ],
        response,
    )
        .into_response()
}
