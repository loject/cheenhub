//! Проверки одноразового получения и отмены desktop OAuth.

use super::{AuthStore, InMemoryAuthStore};
use crate::features::auth::domain::{
    DesktopOAuthAttempt, DesktopOAuthIdentity, DesktopOAuthStatus,
};
use chrono::{Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;

async fn attempt(store: &InMemoryAuthStore) -> DesktopOAuthAttempt {
    let now = Utc::now();
    let expires_at = now + Duration::minutes(5);
    let state_id = store
        .insert_oauth_state(
            "state-hash".into(),
            "nonce".into(),
            "login".into(),
            None,
            now,
            expires_at,
        )
        .await
        .unwrap();
    let attempt = DesktopOAuthAttempt {
        id: Uuid::new_v4(),
        oauth_state_id: state_id,
        secret_hash: "secret-hash".into(),
        expires_at,
    };
    store
        .insert_desktop_oauth_attempt(attempt.clone())
        .await
        .unwrap();
    attempt
}

fn identity() -> DesktopOAuthIdentity {
    DesktopOAuthIdentity {
        subject: "google-subject".into(),
        email: "example@example.test".into(),
        display_name: None,
    }
}

async fn ready(store: &InMemoryAuthStore, attempt: &DesktopOAuthAttempt) -> Uuid {
    assert!(
        store
            .finish_desktop_oauth_attempt(
                &attempt.id,
                "desktop_login".into(),
                None,
                identity(),
                Utc::now()
            )
            .await
            .unwrap()
    );
    store
        .find_active_oauth_handoff(&attempt.secret_hash, Utc::now())
        .await
        .unwrap()
        .unwrap()
        .id
}

#[tokio::test]
async fn desktop_status_requires_secret_and_does_not_consume_result() {
    let store = InMemoryAuthStore::default();
    let attempt = attempt(&store).await;
    assert_eq!(
        store
            .desktop_oauth_status(&attempt.id, "wrong", Utc::now())
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        store
            .desktop_oauth_status(&attempt.id, &attempt.secret_hash, Utc::now())
            .await
            .unwrap(),
        Some(DesktopOAuthStatus::Pending)
    );
    assert!(
        !store
            .cancel_desktop_oauth_attempt(&attempt.id, "wrong", Utc::now())
            .await
            .unwrap()
    );
    let handoff = ready(&store, &attempt).await;
    for _ in 0..2 {
        assert_eq!(
            store
                .desktop_oauth_status(&attempt.id, &attempt.secret_hash, Utc::now())
                .await
                .unwrap(),
            Some(DesktopOAuthStatus::Ready)
        );
    }
    assert!(
        store
            .desktop_oauth_identity_for_handoff(&handoff)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .consume_oauth_handoff(&handoff, Utc::now())
            .await
            .unwrap()
    );
    assert!(
        !store
            .consume_oauth_handoff(&handoff, Utc::now())
            .await
            .unwrap()
    );
    assert_eq!(
        store
            .desktop_oauth_identity_for_handoff(&handoff)
            .await
            .unwrap()
            .unwrap()
            .subject,
        "google-subject"
    );
}

#[tokio::test]
async fn desktop_cancellation_before_callback_blocks_result() {
    let store = InMemoryAuthStore::default();
    let attempt = attempt(&store).await;
    assert!(
        store
            .cancel_desktop_oauth_attempt(&attempt.id, &attempt.secret_hash, Utc::now())
            .await
            .unwrap()
    );
    assert!(
        !store
            .finish_desktop_oauth_attempt(
                &attempt.id,
                "desktop_login".into(),
                None,
                identity(),
                Utc::now()
            )
            .await
            .unwrap()
    );
    assert!(
        !store
            .fail_desktop_oauth_attempt(&attempt.id, "error".into(), Utc::now())
            .await
            .unwrap()
    );
    assert_eq!(
        store
            .desktop_oauth_attempt_by_state_hash("state-hash")
            .await
            .unwrap(),
        Some(attempt.id)
    );
    assert!(
        store
            .find_active_oauth_handoff(&attempt.secret_hash, Utc::now())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn desktop_cancellation_after_callback_revokes_handoff() {
    let store = InMemoryAuthStore::default();
    let attempt = attempt(&store).await;
    let handoff = ready(&store, &attempt).await;
    assert!(
        store
            .cancel_desktop_oauth_attempt(&attempt.id, &attempt.secret_hash, Utc::now())
            .await
            .unwrap()
    );
    assert!(
        !store
            .consume_oauth_handoff(&handoff, Utc::now())
            .await
            .unwrap()
    );
    assert!(
        store
            .find_active_oauth_handoff(&attempt.secret_hash, Utc::now())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .desktop_oauth_identity_for_handoff(&handoff)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn desktop_expiry_blocks_callback_and_claim() {
    let store = InMemoryAuthStore::default();
    let attempt = attempt(&store).await;
    assert!(
        !store
            .finish_desktop_oauth_attempt(
                &attempt.id,
                "desktop_login".into(),
                None,
                identity(),
                attempt.expires_at
            )
            .await
            .unwrap()
    );
    let handoff = ready(&store, &attempt).await;
    assert_eq!(
        store
            .desktop_oauth_status(&attempt.id, &attempt.secret_hash, attempt.expires_at)
            .await
            .unwrap(),
        Some(DesktopOAuthStatus::Expired)
    );
    assert!(
        !store
            .consume_oauth_handoff(&handoff, attempt.expires_at)
            .await
            .unwrap()
    );
    assert!(
        !store
            .cancel_desktop_oauth_attempt(&attempt.id, &attempt.secret_hash, attempt.expires_at)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn desktop_failure_is_terminal() {
    let store = InMemoryAuthStore::default();
    let attempt = attempt(&store).await;
    assert!(
        store
            .fail_desktop_oauth_attempt(&attempt.id, "Не удалось войти".into(), Utc::now())
            .await
            .unwrap()
    );
    assert!(
        !store
            .finish_desktop_oauth_attempt(
                &attempt.id,
                "desktop_login".into(),
                None,
                identity(),
                Utc::now()
            )
            .await
            .unwrap()
    );
    assert_eq!(
        store
            .desktop_oauth_status(&attempt.id, &attempt.secret_hash, Utc::now())
            .await
            .unwrap(),
        Some(DesktopOAuthStatus::Failed("Не удалось войти".into()))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_cancel_and_claim_have_exactly_one_winner() {
    for _ in 0..32 {
        let store = Arc::new(InMemoryAuthStore::default());
        let attempt = attempt(&store).await;
        let handoff = ready(&store, &attempt).await;
        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let cancel_store = Arc::clone(&store);
        let cancel_barrier = Arc::clone(&barrier);
        let cancel = tokio::spawn(async move {
            cancel_barrier.wait().await;
            cancel_store
                .cancel_desktop_oauth_attempt(&attempt.id, &attempt.secret_hash, Utc::now())
                .await
                .unwrap()
        });
        let claim = tokio::spawn(async move {
            barrier.wait().await;
            store
                .consume_oauth_handoff(&handoff, Utc::now())
                .await
                .unwrap()
        });
        assert_ne!(cancel.await.unwrap(), claim.await.unwrap());
    }
}
