//! Регрессия конкурирующей первой записи настроек регистрации в PostgreSQL.

use std::{sync::Arc, time::Duration};

use chrono::Utc;
use sea_orm::{
    ConnectionTrait, Database, DbBackend, EntityTrait, ExprTrait, Set, Statement, TransactionTrait,
    sea_query::{Alias, Expr, Query},
};
use uuid::Uuid;

use super::super::{
    HostSettingsStore, PostgresHostSettingsStore, entities::host_registration_settings,
};
use crate::features::auth::domain::RegistrationLegalAcceptance;
use crate::features::auth::infrastructure::{AuthStore, PostgresAuthStore};
use crate::features::host_settings::domain::{HostRegistrationSettings, REGISTRATION_SETTINGS_ID};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "Нужна изолированная cheenhub_registration_test с миграциями и CHEENHUB_TEST_DATABASE_URL"]
async fn postgres_concurrent_first_saves_preserve_settings_and_audit() {
    let database_url = std::env::var("CHEENHUB_TEST_DATABASE_URL")
        .expect("set CHEENHUB_TEST_DATABASE_URL for the disposable database");
    assert_eq!(
        url::Url::parse(&database_url).unwrap().path(),
        "/cheenhub_registration_test",
        "use only the disposable registration database"
    );
    let db = Database::connect(&database_url).await.unwrap();
    host_registration_settings::Entity::delete_many()
        .exec(&db)
        .await
        .unwrap();
    let label = Uuid::new_v4().simple().to_string();
    let email = format!("{label}@example.com");
    let now = Utc::now();
    let owner = PostgresAuthStore::new(db.clone())
        .insert_user(
            format!("owner_{}", &label[..12]),
            email.clone(),
            email,
            None,
            RegistrationLegalAcceptance {
                acceptance_source: "registration_test".to_owned(),
                terms_version: "test".to_owned(),
                privacy_policy_version: "test".to_owned(),
                personal_data_consent_version: "test".to_owned(),
            },
            now,
        )
        .await
        .unwrap();
    let store = Arc::new(PostgresHostSettingsStore::new(db.clone()));
    assert_eq!(
        store.load_registration_settings().await.unwrap(),
        HostRegistrationSettings::default()
    );

    // Незакоммиченная строка невидима SELECT, но блокирует INSERT по уникальному ключу.
    // Обе операции гарантированно попадают в ветку первой записи старой реализации.
    let guard = db.begin().await.unwrap();
    host_registration_settings::Entity::insert(host_registration_settings::ActiveModel {
        id: Set(REGISTRATION_SETTINGS_ID),
        registration_enabled: Set(true),
        email_password_registration_enabled: Set(true),
        updated_by_user_id: Set(None),
        updated_at: Set(now - chrono::Duration::seconds(1)),
    })
    .exec(&guard)
    .await
    .unwrap();
    let settings = HostRegistrationSettings {
        registration_enabled: false,
        email_password_registration_enabled: false,
    };
    let mut attempts = Vec::new();
    for _ in 0..2 {
        let store = store.clone();
        attempts.push(tokio::spawn(async move {
            store
                .save_registration_settings(settings, owner.id, now)
                .await
        }));
    }

    // pg_stat_activity подтверждает ожидание обоих INSERT; задержка не задаёт порядок гонки.
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let waiting = db
                .query_one_raw(Statement::from_string(
                    DbBackend::Postgres,
                    "SELECT count(*) AS waiting FROM pg_stat_activity \
                     WHERE datname = current_database() AND pid <> pg_backend_pid() \
                     AND wait_event_type = 'Lock' \
                     AND query LIKE 'INSERT INTO %host_registration_settings%'",
                ))
                .await
                .unwrap()
                .unwrap()
                .try_get::<i64>("", "waiting")
                .unwrap();
            if waiting == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("both saves must wait for the uncommitted singleton");
    guard.commit().await.unwrap();
    for attempt in attempts {
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(10), attempt)
                .await
                .unwrap()
                .unwrap()
                .expect("concurrent first save must succeed"),
            settings
        );
    }

    let saved = host_registration_settings::Entity::find_by_id(REGISTRATION_SETTINGS_ID)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(!saved.registration_enabled);
    assert!(!saved.email_password_registration_enabled);
    assert_eq!(saved.updated_by_user_id, Some(owner.id));
    assert_eq!(saved.updated_at.timestamp_micros(), now.timestamp_micros());
    assert_eq!(store.load_registration_settings().await.unwrap(), settings);

    host_registration_settings::Entity::delete_by_id(REGISTRATION_SETTINGS_ID)
        .exec(&db)
        .await
        .unwrap();
    let cleanup = Query::delete()
        .from_table(Alias::new("users"))
        .and_where(Expr::col(Alias::new("id")).eq(owner.id))
        .to_owned();
    db.execute(&cleanup).await.unwrap();
}
