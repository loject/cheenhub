//! Проверки сроков, одноразовости восстановления и отзыва доступа.
use super::{AuthStore, InMemoryAuthStore};
use crate::features::auth::domain::RegistrationLegalAcceptance;
use chrono::{Duration, Utc};

#[tokio::test]
async fn tombstone_revokes_access_and_restores_only_once_before_deadline() {
    let store = InMemoryAuthStore::default();
    let now = Utc::now();
    let user = store
        .insert_user(
            "delete_me".into(),
            "delete@example.com".into(),
            "delete@example.com".into(),
            Some("hash".into()),
            acceptance(),
            now,
        )
        .await
        .unwrap();
    let session = store
        .create_session(
            &user.id,
            "refresh".into(),
            None,
            now,
            now + Duration::days(60),
        )
        .await
        .unwrap();
    store
        .insert_password_reset_token(&user.id, "reset".into(), now, now + Duration::hours(1))
        .await
        .unwrap();
    assert!(
        store
            .begin_account_deletion(&user.id, "restore".into(), now, now + Duration::days(30))
            .await
            .unwrap()
    );
    assert!(!store.session_is_active(&session, now).await.unwrap());
    assert!(
        store
            .find_active_refresh("refresh", now)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .find_active_password_reset_token("reset", now)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .create_session(
                &user.id,
                "new-refresh".into(),
                None,
                now,
                now + Duration::days(1)
            )
            .await
            .is_err()
    );
    assert!(
        store
            .search_users_by_nickname("delete", 10)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store
            .find_user_by_id(&user.id)
            .await
            .unwrap()
            .unwrap()
            .nickname,
        "Удалённый пользователь"
    );
    assert!(!store.restore_account("wrong", now).await.unwrap());
    assert!(
        store
            .restore_account(
                "restore",
                now + Duration::days(30) - Duration::milliseconds(1)
            )
            .await
            .unwrap()
    );
    assert!(!store.restore_account("restore", now).await.unwrap());
    assert!(!store.session_is_active(&session, now).await.unwrap());
    assert_eq!(
        store
            .find_user_by_id(&user.id)
            .await
            .unwrap()
            .unwrap()
            .nickname,
        "delete_me"
    );
}

#[tokio::test]
async fn deadline_is_exclusive_and_finalization_retains_uuid_tombstone() {
    let store = InMemoryAuthStore::default();
    let now = Utc::now();
    let deadline = now + Duration::days(30);
    let user = store
        .insert_user(
            "expired".into(),
            "expired@example.com".into(),
            "expired@example.com".into(),
            Some("hash".into()),
            acceptance(),
            now,
        )
        .await
        .unwrap();
    assert!(
        store
            .begin_account_deletion(&user.id, "restore".into(), now, deadline)
            .await
            .unwrap()
    );
    assert!(!store.restore_account("restore", deadline).await.unwrap());
    assert_eq!(
        store
            .finalize_expired_account_deletions(deadline - Duration::milliseconds(1))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        store
            .finalize_expired_account_deletions(deadline)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        store
            .finalize_expired_account_deletions(deadline)
            .await
            .unwrap(),
        0
    );
    assert!(store.account_deletion(&user.id).await.unwrap().is_some());
    assert!(
        store
            .find_user_by_email("expired@example.com")
            .await
            .unwrap()
            .is_none()
    );
    let remaining = store.find_user_by_id(&user.id).await.unwrap().unwrap();
    assert_eq!(remaining.id, user.id);
    assert!(remaining.password_hash.is_none());
    assert!(!store.restore_account("restore", now).await.unwrap());
}

#[tokio::test]
async fn stale_profile_and_oauth_writes_cannot_repopulate_tombstone() {
    let store = InMemoryAuthStore::default();
    let now = Utc::now();
    let deadline = now + Duration::days(30);
    let user = store
        .insert_user(
            "stale".into(),
            "stale@example.com".into(),
            "stale@example.com".into(),
            Some("hash".into()),
            acceptance(),
            now,
        )
        .await
        .unwrap();
    let session_id = store
        .create_session(&user.id, "refresh".into(), None, now, deadline)
        .await
        .unwrap();
    store
        .begin_account_deletion(&user.id, "restore".into(), now, deadline)
        .await
        .unwrap();
    for time in [now, deadline] {
        store
            .finalize_expired_account_deletions(time)
            .await
            .unwrap();
        assert!(
            store
                .update_user_nickname(
                    &user.id,
                    &session_id,
                    "resurrected".into(),
                    time,
                    Duration::zero(),
                )
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            store
                .update_user_avatar_image_id(&user.id, uuid::Uuid::new_v4(), time,)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            store
                .change_user_password(&user.id, &session_id, "new-hash".into(), time,)
                .await
                .is_err()
        );
        assert!(
            store
                .insert_oauth_account(
                    &user.id,
                    "google".into(),
                    "subject".into(),
                    "stale@example.com".into(),
                    Some("Name".into()),
                    time,
                )
                .await
                .is_err()
        );
    }
    let remaining = store.find_user_by_id(&user.id).await.unwrap().unwrap();
    assert_eq!(remaining.email, format!("deleted:{}", user.id));
    assert!(remaining.password_hash.is_none());
    assert!(remaining.avatar_image_id.is_none());
    assert!(
        store
            .list_oauth_accounts(&user.id)
            .await
            .unwrap()
            .is_empty()
    );
}

fn acceptance() -> RegistrationLegalAcceptance {
    RegistrationLegalAcceptance {
        acceptance_source: "test".into(),
        terms_version: "1".into(),
        privacy_policy_version: "1".into(),
        personal_data_consent_version: "1".into(),
    }
}

#[tokio::test]
async fn restoring_one_user_preserves_others_and_allows_a_new_deletion() {
    let store = InMemoryAuthStore::default();
    let now = Utc::now();
    let mut users = Vec::new();
    for name in ["first", "second", "active"] {
        users.push(
            store
                .insert_user(
                    name.to_owned(),
                    format!("{name}@example.com"),
                    format!("{name}@example.com"),
                    Some("hash".to_owned()),
                    acceptance(),
                    now,
                )
                .await
                .unwrap(),
        );
    }
    let deadline = now + Duration::days(30);
    for (user, token) in [(&users[0], "first-token"), (&users[1], "second-token")] {
        assert!(
            store
                .begin_account_deletion(&user.id, token.to_owned(), now, deadline)
                .await
                .unwrap()
        );
    }
    assert!(store.restore_account("first-token", now).await.unwrap());
    assert!(
        store
            .account_deletion(&users[0].id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .account_deletion(&users[1].id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .account_deletion(&users[2].id)
            .await
            .unwrap()
            .is_none()
    );
    let requested_again = now + Duration::days(1);
    assert!(
        store
            .begin_account_deletion(
                &users[0].id,
                "new-first-token".to_owned(),
                requested_again,
                requested_again + Duration::days(30),
            )
            .await
            .unwrap()
    );
    assert!(
        !store
            .restore_account("first-token", requested_again)
            .await
            .unwrap()
    );
    assert_eq!(
        store
            .finalize_expired_account_deletions(deadline)
            .await
            .unwrap(),
        1
    );
    assert!(
        store
            .restore_account("new-first-token", deadline)
            .await
            .unwrap()
    );
    assert!(
        !store
            .restore_account("second-token", deadline)
            .await
            .unwrap()
    );
    assert_eq!(
        store
            .find_user_by_id(&users[2].id)
            .await
            .unwrap()
            .unwrap()
            .nickname,
        "active"
    );
}
