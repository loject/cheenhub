//! Проверки страниц зарегистрированных пользователей и доступа к их данным.

use super::*;
use crate::features::auth::domain::RegistrationLegalAcceptance;
use crate::features::host_settings::owners::list_users;

#[tokio::test]
async fn registered_users_are_paginated_with_email_and_registration_date() {
    let (state, owner, _) = host_with_owner_and_guest().await;
    let registered_at = chrono::DateTime::parse_from_rfc3339("2026-01-01T12:00:00Z")
        .unwrap()
        .to_utc();
    for index in 0..26 {
        let email = format!("page{index}@example.com");
        state
            .auth_store
            .insert_user(
                format!("page{index}"),
                email.clone(),
                email,
                None,
                RegistrationLegalAcceptance {
                    acceptance_source: "password".into(),
                    terms_version: "v1".into(),
                    privacy_policy_version: "v1".into(),
                    personal_data_consent_version: "v1".into(),
                },
                registered_at,
            )
            .await
            .unwrap();
    }

    let first = list_users(&state, &owner.access_token, 1).await.unwrap();
    let second = list_users(&state, &owner.access_token, 2).await.unwrap();

    assert_eq!(first.total, 28);
    assert_eq!(first.users.len(), 25);
    assert_eq!(second.users.len(), 3);
    assert_eq!(second.page, 2);
    assert!(
        first
            .users
            .iter()
            .all(|a| second.users.iter().all(|b| a.user_id != b.user_id))
    );
    assert!(
        second
            .users
            .iter()
            .all(|user| user.email.ends_with("@example.com")
                && user.registered_at == "2026-01-01T12:00:00Z")
    );
    assert_eq!(
        list_users(&state, &owner.access_token, u64::MAX)
            .await
            .unwrap(),
        second
    );
}

#[tokio::test]
async fn ordinary_user_cannot_list_registered_users() {
    let (state, _, _) = host_with_owner_and_guest().await;
    let outsider = register(&state, "outsider", "outsider@example.com").await;

    let result = list_users(&state, &outsider.access_token, 1).await;

    assert!(matches!(result, Err(HostSettingsError::Forbidden(_))));
}

#[tokio::test]
async fn promoted_owner_can_read_registered_users_without_management_rights() {
    let (state, owner, _) = host_with_owner_and_guest().await;
    let second = register(&state, "second_owner", "second@example.com").await;
    let owners = grant_owner(
        &state,
        &owner.access_token,
        GrantHostOwnerRequest {
            user: second.user.id,
        },
    )
    .await
    .unwrap();

    let users = list_users(&state, &second.access_token, 1).await.unwrap();
    let own_view = list_owners(&state, &second.access_token).await.unwrap();

    assert_eq!(users.total, 3);
    assert!(owners.owners[0].is_initial_owner);
    assert!(
        !own_view
            .owners
            .iter()
            .any(|entry| entry.is_current_user && entry.is_initial_owner)
    );
}

#[tokio::test]
async fn zero_page_is_rejected() {
    let (state, owner, _) = host_with_owner_and_guest().await;

    let result = list_users(&state, &owner.access_token, 0).await;

    assert!(matches!(result, Err(HostSettingsError::BadRequest(_))));
}
