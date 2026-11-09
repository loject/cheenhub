//! Проверки порядка страниц пользователей и исключения обезличенных аккаунтов.

use super::super::{AuthStore, InMemoryAuthStore};
use crate::features::auth::domain::{RegistrationLegalAcceptance, UserAccount};
use chrono::{DateTime, Duration, Utc};

async fn insert(store: &InMemoryAuthStore, name: &str, at: DateTime<Utc>) -> UserAccount {
    let email = format!("{name}@example.com");
    store
        .insert_user(
            name.into(),
            email.clone(),
            email,
            None,
            RegistrationLegalAcceptance {
                acceptance_source: "password".into(),
                terms_version: "v1".into(),
                privacy_policy_version: "v1".into(),
                personal_data_consent_version: "v1".into(),
            },
            at,
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn pages_follow_registration_order_and_skip_finalized_accounts() {
    let store = InMemoryAuthStore::default();
    let now = Utc::now();
    let oldest = insert(&store, "oldest", now - Duration::days(40)).await;
    let deleted = insert(&store, "deleted", now - Duration::days(39)).await;
    let newest = insert(&store, "newest", now).await;
    store
        .begin_account_deletion(
            &deleted.id,
            "deleted-token".into(),
            now - Duration::days(31),
            now - Duration::days(1),
        )
        .await
        .unwrap();
    store.finalize_expired_account_deletions(now).await.unwrap();

    let first = store.list_registered_users(0, 1).await.unwrap();
    let second = store.list_registered_users(1, 1).await.unwrap();
    let outside = store.list_registered_users(2, 1).await.unwrap();

    assert_eq!(first[0].id, newest.id);
    assert_eq!(second[0].id, oldest.id);
    assert!(outside.is_empty());
    assert_eq!(store.count_registered_users().await.unwrap(), 2);
}

#[tokio::test]
async fn equal_registration_times_use_uuid_order_for_stable_pages() {
    let store = InMemoryAuthStore::default();
    let at = Utc::now();
    let a = insert(&store, "alice", at).await;
    let b = insert(&store, "bob", at).await;

    let first = store.list_registered_users(0, 1).await.unwrap();
    let second = store.list_registered_users(1, 1).await.unwrap();

    assert_eq!(first[0].id, a.id.min(b.id));
    assert_eq!(second[0].id, a.id.max(b.id));
}
