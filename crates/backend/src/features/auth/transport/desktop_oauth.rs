//! HTTP-адаптер ожидания Google OAuth в настольном приложении.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use cheenhub_contracts::rest::{GoogleDesktopAuthRequest, OAuthStartRequest};

use crate::features::auth::{application, error::AuthError};
use crate::state::AppState;

/// Запускает вход Google с возвратом результата настольному приложению.
pub(crate) async fn start(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<OAuthStartRequest>,
) -> Response {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    private_response(
        application::start_desktop_oauth(&state, token, request)
            .await
            .map(Json),
    )
}

/// Получает состояние входа без раскрытия результата сторонним клиентам.
pub(crate) async fn poll(
    State(state): State<AppState>,
    Json(request): Json<GoogleDesktopAuthRequest>,
) -> Response {
    private_response(
        application::poll_desktop_oauth(&state, request)
            .await
            .map(Json),
    )
}

/// Отменяет вход из исходного настольного приложения.
pub(crate) async fn cancel(
    State(state): State<AppState>,
    Json(request): Json<GoogleDesktopAuthRequest>,
) -> Response {
    private_response(
        application::cancel_desktop_oauth(&state, request)
            .await
            .map(|()| StatusCode::NO_CONTENT),
    )
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
