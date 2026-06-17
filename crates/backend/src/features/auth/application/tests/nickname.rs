//! Тесты приложения обновления никнейма.

use cheenhub_contracts::rest::UpdateCurrentUserRequest;
use chrono::{Duration, Utc};

use super::{registered_user, state};
use crate::features::auth::application::{me, update_current_user};
use crate::features::auth::error::AuthError;

#[tokio::test]
async fn update_current_user_changes_nickname_after_cooldown() {
    let state = state();
    let auth = registered_user(&state, "ready_user", "ready-user@example.com").await;
    backdate_nickname_update(&state, &auth.user.id, "ready_user").await;

    let updated = update_current_user(
        &state,
        &auth.access_token,
        UpdateCurrentUserRequest {
            nickname: "renamed_user".to_owned(),
        },
    )
    .await
    .expect("nickname update should succeed");

    assert_eq!(updated.nickname, "renamed_user");
    let current_user = me(&state, &auth.access_token)
        .await
        .expect("session should remain active");
    assert_eq!(current_user.nickname, "renamed_user");
}

#[tokio::test]
async fn update_current_user_rejects_taken_nickname() {
    let state = state();
    registered_user(&state, "taken_user", "taken-user@example.com").await;
    let auth = registered_user(&state, "rename_conflict", "rename-conflict@example.com").await;
    backdate_nickname_update(&state, &auth.user.id, "rename_conflict").await;

    let result = update_current_user(
        &state,
        &auth.access_token,
        UpdateCurrentUserRequest {
            nickname: "taken_user".to_owned(),
        },
    )
    .await;

    assert!(matches!(result, Err(AuthError::Conflict(_))));
}

#[tokio::test]
async fn update_current_user_rejects_invalid_nickname() {
    let state = state();
    let auth = registered_user(&state, "invalid_rename", "invalid-rename@example.com").await;

    let result = update_current_user(
        &state,
        &auth.access_token,
        UpdateCurrentUserRequest {
            nickname: "no spaces".to_owned(),
        },
    )
    .await;

    assert!(matches!(result, Err(AuthError::BadRequest(_))));
}

#[tokio::test]
async fn update_current_user_rejects_cooldown() {
    let state = state();
    let auth = registered_user(&state, "cooldown_user", "cooldown-user@example.com").await;

    let result = update_current_user(
        &state,
        &auth.access_token,
        UpdateCurrentUserRequest {
            nickname: "too_soon".to_owned(),
        },
    )
    .await;

    assert!(matches!(result, Err(AuthError::RateLimited(_))));
}

#[tokio::test]
async fn update_current_user_allows_second_change_after_cooldown() {
    let state = state();
    let auth = registered_user(&state, "second_ready", "second-ready@example.com").await;
    backdate_nickname_update(&state, &auth.user.id, "second_ready").await;
    update_current_user(
        &state,
        &auth.access_token,
        UpdateCurrentUserRequest {
            nickname: "first_change".to_owned(),
        },
    )
    .await
    .expect("first nickname update should succeed");
    backdate_nickname_update(&state, &auth.user.id, "first_change").await;

    let updated = update_current_user(
        &state,
        &auth.access_token,
        UpdateCurrentUserRequest {
            nickname: "second_change".to_owned(),
        },
    )
    .await
    .expect("second nickname update should succeed after cooldown");

    assert_eq!(updated.nickname, "second_change");
}

async fn backdate_nickname_update(state: &crate::state::AppState, user_id: &str, nickname: &str) {
    let user_id = uuid::Uuid::parse_str(user_id).expect("user id should parse");
    state
        .auth_store
        .update_user_nickname(
            &user_id,
            &uuid::Uuid::new_v4(),
            nickname.to_owned(),
            Utc::now() - Duration::days(8),
            Duration::zero(),
        )
        .await
        .expect("nickname timestamp should update");
}
