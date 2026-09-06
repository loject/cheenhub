//! Проверка очистки на изолированной PostgreSQL с реальными внешними ключами.

use super::{attempts, cleanup, oauth_handoffs, oauth_states};
use crate::features::auth::{
    domain::{DesktopOAuthAttempt, DesktopOAuthIdentity},
    infrastructure::{AuthStore, PostgresAuthStore},
    security::refresh_token,
};
use chrono::{Duration, Utc};
use sea_orm::{ColumnTrait, Database, EntityTrait, QueryFilter, sea_query::Expr};
use uuid::Uuid;

#[tokio::test]
#[ignore = "Нужна изолированная cheenhub_oauth_test с миграциями и CHEENHUB_TEST_DATABASE_URL"]
async fn cleanup_removes_expired_desktop_data_and_preserves_live_and_web_flows() {
    let database_url = std::env::var("CHEENHUB_TEST_DATABASE_URL").expect("test database URL");
    let parsed = url::Url::parse(&database_url).expect("valid test database URL");
    assert_eq!(parsed.path(), "/cheenhub_oauth_test");
    let database = Database::connect(database_url)
        .await
        .expect("connect isolated PostgreSQL");
    let store = PostgresAuthStore::new(database.clone());
    // Прошлый cutoff не затрагивает попытки других тестов, запущенных параллельно.
    let now = Utc::now();
    let cutoff = now - Duration::minutes(10);
    let mut expired_states = Vec::new();
    let mut expired_handoffs = Vec::new();
    let mut expired_attempts = Vec::new();
    let mut live_attempt = None;
    let mut claimed_identity = None;
    for status in ["pending", "ready", "cancelled", "failed", "claimed", "live"] {
        let state_id = store
            .insert_oauth_state(
                refresh_token::hash(&refresh_token::generate()),
                "nonce".to_owned(),
                "login".to_owned(),
                None,
                now,
                now + Duration::hours(1),
            )
            .await
            .unwrap();
        let attempt = DesktopOAuthAttempt {
            id: Uuid::new_v4(),
            oauth_state_id: state_id,
            secret_hash: refresh_token::hash(&refresh_token::generate()),
            expires_at: now + Duration::minutes(5),
        };
        store
            .insert_desktop_oauth_attempt(attempt.clone())
            .await
            .unwrap();
        if matches!(status, "ready" | "cancelled" | "claimed" | "live") {
            assert!(
                store
                    .finish_desktop_oauth_attempt(
                        &attempt.id,
                        "desktop_login".to_owned(),
                        None,
                        DesktopOAuthIdentity {
                            subject: "cleanup-subject".to_owned(),
                            email: "cleanup@example.test".to_owned(),
                            display_name: None
                        },
                        now,
                    )
                    .await
                    .unwrap()
            );
            let handoff = store
                .find_active_oauth_handoff(&attempt.secret_hash, now)
                .await
                .unwrap()
                .unwrap();
            if status != "live" {
                expired_handoffs.push(handoff.id);
            }
            if status == "cancelled" {
                assert!(
                    store
                        .cancel_desktop_oauth_attempt(&attempt.id, &attempt.secret_hash, now)
                        .await
                        .unwrap()
                );
            } else if status == "claimed" {
                // Завершение держит личность локально до захвата, очистка после него безопасна.
                claimed_identity = store
                    .desktop_oauth_identity_for_handoff(&handoff.id)
                    .await
                    .unwrap();
                assert!(store.consume_oauth_handoff(&handoff.id, now).await.unwrap());
            }
        } else if status == "failed" {
            assert!(
                store
                    .fail_desktop_oauth_attempt(&attempt.id, "Отменено".to_owned(), now)
                    .await
                    .unwrap()
            );
        }
        if status == "live" {
            live_attempt = Some(attempt);
        } else {
            expired_states.push(state_id);
            expired_attempts.push(attempt.id);
            attempts::Entity::update_many()
                .col_expr(attempts::Column::ExpiresAt, Expr::value(cutoff))
                .filter(attempts::Column::Id.eq(attempt.id))
                .exec(&database)
                .await
                .unwrap();
        }
    }
    let web_state = store
        .insert_oauth_state(
            refresh_token::hash(&refresh_token::generate()),
            "web-nonce".to_owned(),
            "login".to_owned(),
            None,
            now,
            cutoff,
        )
        .await
        .unwrap();
    // Проверяем, что после очистки не остаётся даже ещё действующего state desktop-потока.
    assert!(cleanup(&database, cutoff).await.unwrap() >= 5);
    for id in expired_attempts {
        assert!(
            attempts::Entity::find_by_id(id)
                .one(&database)
                .await
                .unwrap()
                .is_none()
        );
    }
    for id in expired_states {
        assert!(
            oauth_states::Entity::find_by_id(id)
                .one(&database)
                .await
                .unwrap()
                .is_none()
        );
    }
    for id in expired_handoffs {
        assert!(
            oauth_handoffs::Entity::find_by_id(id)
                .one(&database)
                .await
                .unwrap()
                .is_none()
        );
    }
    let live = live_attempt.unwrap();
    assert!(
        attempts::Entity::find_by_id(live.id)
            .one(&database)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .find_active_oauth_handoff(&live.secret_hash, now)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        oauth_states::Entity::find_by_id(web_state)
            .one(&database)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(claimed_identity.unwrap().subject, "cleanup-subject");
    assert_eq!(cleanup(&database, cutoff).await.unwrap(), 0);
}
