//! Проверка взаимоисключения создания сервера и удаления аккаунта.

use super::{CreateServerRequest, RegisterRequest, auth_application, create, state};
use uuid::Uuid;

#[tokio::test]
async fn deletion_and_server_creation_cannot_both_succeed() {
    let state = state();
    let auth = auth_application::register(
        &state,
        RegisterRequest {
            nickname: "owner_race".to_owned(),
            email: "owner-race@example.com".to_owned(),
            password: "password123".to_owned(),
            accepts_terms: true,
            accepts_personal_data: true,
        },
    )
    .await
    .unwrap();
    let user_id = Uuid::parse_str(&auth.user.id).unwrap();
    let guard = state
        .auth_store
        .lock_account_lifecycle(&user_id)
        .await
        .unwrap();
    let create_state = state.clone();
    let create_token = auth.access_token.clone();
    let creation = tokio::spawn(async move {
        create(
            &create_state,
            &create_token,
            CreateServerRequest {
                name: "Мой сервер".to_owned(),
            },
        )
        .await
    });
    let delete_state = state.clone();
    let deletion = tokio::spawn(async move {
        auth_application::delete_current_user(&delete_state, &auth.access_token).await
    });
    tokio::task::yield_now().await;
    drop(guard);
    let created = creation.await.unwrap();
    let deleted = deletion.await.unwrap();
    assert_ne!(created.is_ok(), deleted.is_ok());
    let owns_servers = state
        .server_store
        .list_servers(&user_id)
        .await
        .unwrap()
        .iter()
        .any(|access| access.server.owner_user_id == user_id);
    let is_deleted = state
        .auth_store
        .account_deletion(&user_id)
        .await
        .unwrap()
        .is_some();
    assert_ne!(owns_servers, is_deleted);
}
