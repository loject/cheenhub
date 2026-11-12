//! REST-операции настройки входящих контактов и блокировки пользователей.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use cheenhub_contracts::rest::{ListBlockedUsersResponse, UserPrivacySettings};

use super::authenticated_user_id;
use crate::features::social::support::parse_id;
use crate::features::social::{application, error::SocialError};
use crate::state::AppState;

/// Возвращает три настройки приватности текущего пользователя.
pub(crate) async fn get_privacy_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<UserPrivacySettings>, SocialError> {
    let user_id = authenticated_user_id(&state, &headers).await?;
    application::get_privacy_settings(&state, &user_id)
        .await
        .map(Json)
}

/// Сохраняет настройки приватности.
pub(crate) async fn update_privacy_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UserPrivacySettings>,
) -> Result<Json<UserPrivacySettings>, SocialError> {
    let user_id = authenticated_user_id(&state, &headers).await?;
    application::update_privacy_settings(&state, &user_id, request)
        .await
        .map(Json)
}

/// Возвращает активные блокировки текущего пользователя.
pub(crate) async fn list_blocked_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ListBlockedUsersResponse>, SocialError> {
    let user_id = authenticated_user_id(&state, &headers).await?;
    application::list_blocked_users(&state, &user_id)
        .await
        .map(Json)
}

/// Устанавливает блокировку указанного пользователя.
pub(crate) async fn block_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target): Path<String>,
) -> Result<StatusCode, SocialError> {
    let user_id = authenticated_user_id(&state, &headers).await?;
    let target = parse_id(&target, "Пользователь не найден.")?;
    application::block_user(&state, &user_id, &target).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Удаляет активную блокировку без восстановления дружбы.
pub(crate) async fn unblock_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target): Path<String>,
) -> Result<StatusCode, SocialError> {
    let user_id = authenticated_user_id(&state, &headers).await?;
    let target = parse_id(&target, "Пользователь не найден.")?;
    application::unblock_user(&state, &user_id, &target).await?;
    Ok(StatusCode::NO_CONTENT)
}
