//! REST-адаптер удаления и восстановления аккаунта.

use crate::features::auth::{application, error::AuthError};
use crate::state::AppState;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use cheenhub_contracts::rest::{AccountDeletionResponse, AccountRestoreRequest};

/// Принимает запрос удаления текущего аккаунта.
pub(crate) async fn delete_current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AccountDeletionResponse>, AuthError> {
    application::delete_current_user(&state, super::handlers::bearer_token(&headers)?)
        .await
        .map(Json)
}

/// Подтверждает восстановление аккаунта отдельным POST-запросом.
pub(crate) async fn restore_account(
    State(state): State<AppState>,
    Json(request): Json<AccountRestoreRequest>,
) -> Result<StatusCode, AuthError> {
    application::restore_account(&state, request).await?;
    Ok(StatusCode::NO_CONTENT)
}
