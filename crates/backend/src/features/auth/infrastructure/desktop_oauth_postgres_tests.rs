//! Проверки desktop OAuth на отдельной PostgreSQL с применёнными миграциями.

use chrono::{Duration, Utc};
use sea_orm::Database;
use uuid::Uuid;

use super::{AuthStore, PostgresAuthStore};
use crate::features::auth::domain::{
    DesktopOAuthAttempt, DesktopOAuthIdentity, DesktopOAuthStatus,
};
use crate::features::auth::security::refresh_token;

async fn attempt(store: &dyn AuthStore) -> (DesktopOAuthAttempt, String) {
    let now = Utc::now();
    let state_hash = refresh_token::hash(&refresh_token::generate());
    let oauth_state_id = store
        .insert_oauth_state(
            state_hash.clone(),
            refresh_token::generate(),
            "login".to_owned(),
            None,
            now,
            now + Duration::minutes(5),
        )
        .await
        .expect("insert OAuth state");
    let attempt = DesktopOAuthAttempt {
        id: Uuid::new_v4(),
        oauth_state_id,
        secret_hash: refresh_token::hash(&refresh_token::generate()),
        expires_at: now + Duration::minutes(5),
    };
    store
        .insert_desktop_oauth_attempt(attempt.clone())
        .await
        .expect("insert desktop attempt");
    (attempt, state_hash)
}

fn identity() -> DesktopOAuthIdentity {
    DesktopOAuthIdentity {
        subject: "postgres-test-subject".to_owned(),
        email: "postgres-test@example.com".to_owned(),
        display_name: Some("Postgres Test".to_owned()),
    }
}

async fn finish(store: &dyn AuthStore, attempt: &DesktopOAuthAttempt) -> bool {
    store
        .finish_desktop_oauth_attempt(
            &attempt.id,
            "desktop_login".to_owned(),
            None,
            identity(),
            Utc::now(),
        )
        .await
        .expect("finish desktop callback")
}

async fn status(store: &dyn AuthStore, attempt: &DesktopOAuthAttempt) -> DesktopOAuthStatus {
    store
        .desktop_oauth_status(&attempt.id, &attempt.secret_hash, Utc::now())
        .await
        .expect("read desktop status")
        .expect("known desktop attempt")
}

#[tokio::test]
#[ignore = "Нужна изолированная cheenhub_oauth_test с миграциями и CHEENHUB_TEST_DATABASE_URL"]
async fn postgres_desktop_oauth_lifecycle_and_races() {
    let database_url = std::env::var("CHEENHUB_TEST_DATABASE_URL")
        .expect("set CHEENHUB_TEST_DATABASE_URL for the disposable database");
    let parsed = url::Url::parse(&database_url).expect("valid database URL");
    assert_eq!(
        parsed.path(),
        "/cheenhub_oauth_test",
        "use only the disposable database"
    );
    let database = Database::connect(&database_url)
        .await
        .expect("connect test PostgreSQL");
    let store = PostgresAuthStore::new(database);

    let (first, state_hash) = attempt(&store).await;
    assert_eq!(status(&store, &first).await, DesktopOAuthStatus::Pending);
    assert_eq!(
        store
            .desktop_oauth_attempt_by_state_hash(&state_hash)
            .await
            .expect("state lookup"),
        Some(first.id),
    );
    assert!(
        store
            .desktop_oauth_status(&first.id, "incorrect-secret", Utc::now())
            .await
            .expect("secret check")
            .is_none()
    );
    let orphan = DesktopOAuthAttempt {
        id: Uuid::new_v4(),
        oauth_state_id: Uuid::new_v4(),
        secret_hash: refresh_token::hash(&refresh_token::generate()),
        ..first.clone()
    };
    assert!(
        store.insert_desktop_oauth_attempt(orphan).await.is_err(),
        "FK rejects a missing OAuth state"
    );

    // Другой pool читает ту же попытку без состояния первого процесса в памяти.
    let second_connection = Database::connect(&database_url)
        .await
        .expect("connect second backend");
    let second_store = PostgresAuthStore::new(second_connection);
    assert_eq!(
        status(&second_store, &first).await,
        DesktopOAuthStatus::Pending
    );

    let (left, right) = tokio::join!(finish(&store, &first), finish(&second_store, &first));
    assert_ne!(left, right, "duplicate callbacks have exactly one winner");
    let handoff = store
        .find_active_oauth_handoff(&first.secret_hash, Utc::now())
        .await
        .expect("handoff lookup")
        .expect("ready handoff");
    assert!(
        store
            .desktop_oauth_identity_for_handoff(&handoff.id)
            .await
            .expect("ready identity lookup")
            .is_some()
    );
    let now = Utc::now();
    let (left, right) = tokio::join!(
        store.consume_oauth_handoff(&handoff.id, now),
        second_store.consume_oauth_handoff(&handoff.id, now),
    );
    assert_ne!(
        left.expect("first claim"),
        right.expect("second claim"),
        "exactly one claim"
    );
    assert_eq!(status(&store, &first).await, DesktopOAuthStatus::Claimed);
    let verified = store
        .desktop_oauth_identity_for_handoff(&handoff.id)
        .await
        .expect("identity lookup")
        .expect("claimed identity");
    assert_eq!(verified.subject, identity().subject);
    assert_eq!(verified.email, identity().email);
    assert!(
        !store
            .cancel_desktop_oauth_attempt(&first.id, &first.secret_hash, now)
            .await
            .expect("late cancel")
    );

    for _ in 0..8 {
        let (pending, _) = attempt(&store).await;
        let (_, cancelled) = tokio::join!(
            finish(&store, &pending),
            second_store.cancel_desktop_oauth_attempt(
                &pending.id,
                &pending.secret_hash,
                Utc::now()
            ),
        );
        assert!(cancelled.expect("cancel concurrent callback"));
        assert_eq!(
            status(&store, &pending).await,
            DesktopOAuthStatus::Cancelled
        );
        assert!(
            store
                .find_active_oauth_handoff(&pending.secret_hash, Utc::now())
                .await
                .expect("cancelled handoff lookup")
                .is_none()
        );
        assert!(
            !finish(&store, &pending).await,
            "late callback cannot resurrect cancellation"
        );

        let (ready, _) = attempt(&store).await;
        assert!(finish(&store, &ready).await);
        let handoff = store
            .find_active_oauth_handoff(&ready.secret_hash, Utc::now())
            .await
            .expect("ready lookup")
            .expect("handoff");
        let now = Utc::now();
        let (claimed, cancelled) = tokio::join!(
            store.consume_oauth_handoff(&handoff.id, now),
            second_store.cancel_desktop_oauth_attempt(&ready.id, &ready.secret_hash, now),
        );
        let claimed = claimed.expect("claim racing cancellation");
        assert_ne!(
            claimed,
            cancelled.expect("cancel racing claim"),
            "cancel and claim are mutually exclusive"
        );
        assert_eq!(
            status(&store, &ready).await,
            if claimed {
                DesktopOAuthStatus::Claimed
            } else {
                DesktopOAuthStatus::Cancelled
            }
        );
    }

    let (failed, _) = attempt(&store).await;
    assert!(
        store
            .fail_desktop_oauth_attempt(&failed.id, "Вход отменён в Google.".to_owned(), Utc::now())
            .await
            .expect("fail attempt")
    );
    assert!(matches!(
        status(&store, &failed).await,
        DesktopOAuthStatus::Failed(_)
    ));
    assert!(!finish(&store, &failed).await);

    let (expired, _) = attempt(&store).await;
    let after_expiry = expired.expires_at + Duration::seconds(1);
    assert_eq!(
        store
            .desktop_oauth_status(&expired.id, &expired.secret_hash, after_expiry)
            .await
            .expect("expiry check"),
        Some(DesktopOAuthStatus::Expired)
    );
    assert!(
        store
            .desktop_oauth_status(&expired.id, "incorrect-secret", after_expiry)
            .await
            .expect("expired secret check")
            .is_none()
    );
    assert!(
        !store
            .finish_desktop_oauth_attempt(
                &expired.id,
                "desktop_login".to_owned(),
                None,
                identity(),
                after_expiry
            )
            .await
            .expect("late finish")
    );
    assert!(
        !store
            .cancel_desktop_oauth_attempt(&expired.id, &expired.secret_hash, after_expiry)
            .await
            .expect("late cancel")
    );
}
