//! Проверки атомарного сохранения последнего владельца хоста.

use chrono::Utc;
use uuid::Uuid;

use super::super::{HostSettingsStore, InMemoryHostSettingsStore};
use crate::features::host_settings::domain::{HostOwner, RevokeHostOwnerOutcome};

#[tokio::test]
async fn store_preserves_the_last_owner_on_direct_revocation() {
    let owner_id = Uuid::new_v4();
    let store = InMemoryHostSettingsStore::with_owner(owner_id);

    let result = store.revoke_host_owner(owner_id).await.unwrap();

    assert_eq!(result, RevokeHostOwnerOutcome::LastOwner);
    assert!(store.is_host_owner(owner_id).await.unwrap());
}

#[tokio::test]
async fn concurrent_revocations_leave_exactly_one_owner() {
    let first_id = Uuid::new_v4();
    let second_id = Uuid::new_v4();
    let store = InMemoryHostSettingsStore::with_owner(first_id);
    store
        .grant_host_owner(HostOwner {
            user_id: second_id,
            granted_at: Utc::now(),
            granted_by_user_id: Some(first_id),
        })
        .await
        .unwrap();

    let (first, second) = tokio::join!(
        store.revoke_host_owner(first_id),
        store.revoke_host_owner(second_id),
    );
    let outcomes = [first.unwrap(), second.unwrap()];
    assert!(outcomes.contains(&RevokeHostOwnerOutcome::Revoked));
    assert!(outcomes.contains(&RevokeHostOwnerOutcome::LastOwner));

    assert_eq!(store.load_host_owners().await.unwrap().len(), 1);
}

#[tokio::test]
async fn missing_owner_revocation_preserves_existing_owner() {
    let owner_id = Uuid::new_v4();
    let store = InMemoryHostSettingsStore::with_owner(owner_id);

    let result = store.revoke_host_owner(Uuid::new_v4()).await.unwrap();

    assert_eq!(result, RevokeHostOwnerOutcome::Missing);
    assert!(store.is_host_owner(owner_id).await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "Нужна изолированная cheenhub_owners_test с миграциями и CHEENHUB_TEST_DATABASE_URL"]
async fn postgres_concurrent_revocations_preserve_the_last_owner() {
    use super::super::{PostgresHostSettingsStore, entities::host_owners};
    use crate::features::auth::domain::RegistrationLegalAcceptance;
    use crate::features::auth::infrastructure::{AuthStore, PostgresAuthStore};
    use sea_orm::{
        ConnectionTrait, EntityTrait, ExprTrait, QueryOrder, QuerySelect, TransactionTrait,
        sea_query::{Alias, Expr, LockType, Query},
    };

    let database_url = std::env::var("CHEENHUB_TEST_DATABASE_URL")
        .expect("set CHEENHUB_TEST_DATABASE_URL for the disposable database");
    let parsed = url::Url::parse(&database_url).unwrap();
    assert_eq!(
        parsed.path(),
        "/cheenhub_owners_test",
        "use only the disposable owners database"
    );
    let db = sea_orm::Database::connect(&database_url).await.unwrap();
    host_owners::Entity::delete_many().exec(&db).await.unwrap();
    let auth_store = PostgresAuthStore::new(db.clone());
    let now = Utc::now();
    let mut user_ids = Vec::new();
    for _ in 0..2 {
        let label = Uuid::new_v4().simple().to_string();
        let email = format!("{label}@example.com");
        let user = auth_store
            .insert_user(
                format!("owner_{}", &label[..12]),
                email.clone(),
                email,
                None,
                RegistrationLegalAcceptance {
                    acceptance_source: "owners_test".to_owned(),
                    terms_version: "test".to_owned(),
                    privacy_policy_version: "test".to_owned(),
                    personal_data_consent_version: "test".to_owned(),
                },
                now,
            )
            .await
            .unwrap();
        user_ids.push(user.id);
    }
    let first_id = user_ids[0];
    let second_id = user_ids[1];
    let store = std::sync::Arc::new(PostgresHostSettingsStore::new(db.clone()));
    for user_id in [first_id, second_id] {
        store
            .grant_host_owner(HostOwner {
                user_id,
                granted_at: now,
                granted_by_user_id: None,
            })
            .await
            .unwrap();
    }
    // Обе операции начинают отзыв, пока общий набор владельцев заблокирован.
    let guard = db.begin().await.unwrap();
    host_owners::Entity::find()
        .order_by_asc(host_owners::Column::UserId)
        .lock(LockType::Update)
        .all(&guard)
        .await
        .unwrap();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
    let mut attempts = Vec::new();
    for user_id in [first_id, second_id] {
        let store = store.clone();
        let barrier = barrier.clone();
        attempts.push(tokio::spawn(async move {
            barrier.wait().await;
            store.revoke_host_owner(user_id).await.unwrap()
        }));
    }

    barrier.wait().await;
    guard.commit().await.unwrap();
    let mut outcomes = Vec::new();
    for attempt in attempts {
        outcomes.push(
            tokio::time::timeout(std::time::Duration::from_secs(5), attempt)
                .await
                .unwrap()
                .unwrap(),
        );
    }

    assert!(outcomes.contains(&RevokeHostOwnerOutcome::Revoked));
    assert!(outcomes.contains(&RevokeHostOwnerOutcome::LastOwner));
    let remaining = store.load_host_owners().await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(
        store.revoke_host_owner(remaining[0].user_id).await.unwrap(),
        RevokeHostOwnerOutcome::LastOwner
    );

    // Удаляем только пользователей сценария из заранее выделенной тестовой базы.
    let cleanup = Query::delete()
        .from_table(Alias::new("users"))
        .and_where(Expr::col(Alias::new("id")).is_in(user_ids))
        .to_owned();
    db.execute(&cleanup).await.unwrap();
}
