//! Публичные настройки проверки JWT; приватный ключ остаётся в security.

use axum::{Json, extract::State, http::header};
use cheenhub_contracts::rest::AuthVerificationKey;

use crate::state::AppState;

#[cfg(test)]
mod tests;

/// Отдаёт текущий публичный ключ без авторизации и без HTTP-кеширования.
pub(crate) async fn get(
    State(state): State<AppState>,
) -> (
    [(header::HeaderName, &'static str); 1],
    Json<AuthVerificationKey>,
) {
    tracing::debug!(key_id = %state.auth_keys.key_id, "serving JWT verification key");
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(state.auth_keys.verification_key()),
    )
}
