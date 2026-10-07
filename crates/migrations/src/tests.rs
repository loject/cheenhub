use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};
use sea_orm_migration::MigratorTrait;
use uuid::Uuid;

use crate::Migrator;

#[test]
fn migration_registry_contains_legacy_legal_acceptance_move() {
    let migration_names = Migrator::migrations()
        .iter()
        .map(|migration| migration.name().to_owned())
        .collect::<Vec<_>>();

    assert!(
        migration_names
            .iter()
            .any(|name| { name == "m20261017_000039_move_legacy_legal_acceptance" })
    );
}

#[tokio::test]
#[ignore = "Нужна отдельная пустая PostgreSQL-база cheenhub_legal_migration_test и CHEENHUB_LEGAL_MIGRATION_TEST_DATABASE_URL"]
async fn migration_preserves_legacy_acceptance_timestamps() {
    let database_url = std::env::var("CHEENHUB_LEGAL_MIGRATION_TEST_DATABASE_URL").expect(
        "CHEENHUB_LEGAL_MIGRATION_TEST_DATABASE_URL должен указывать на отдельную тестовую базу",
    );
    let database = Database::connect(&database_url)
        .await
        .expect("тестовая PostgreSQL-база должна быть доступна");
    let database_name = database
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT current_database() AS name".to_owned(),
        ))
        .await
        .expect("имя тестовой базы должно читаться")
        .expect("PostgreSQL должен вернуть имя тестовой базы")
        .try_get::<String>("", "name")
        .expect("имя тестовой базы должно быть строкой");

    assert_eq!(database_name, "cheenhub_legal_migration_test");
    Migrator::fresh(&database)
        .await
        .expect("изолированная тестовая база должна очиститься");
    Migrator::up(&database, Some(38))
        .await
        .expect("предыдущие миграции должны создать исходную схему");

    let equal_timestamp_user = Uuid::new_v4();
    let newer_acceptance_user = Uuid::new_v4();
    let no_acceptance_user = Uuid::new_v4();
    let legacy_at = timestamp("2026-01-02T03:04:05Z");
    let later_at = timestamp("2026-02-03T04:05:06Z");

    insert_user(&database, equal_timestamp_user, "equal", legacy_at).await;
    insert_user(&database, newer_acceptance_user, "newer", legacy_at).await;
    insert_user(&database, no_acceptance_user, "missing", legacy_at).await;
    insert_terms_acceptance(
        &database,
        equal_timestamp_user,
        "2025-12-01",
        timestamp("2025-12-01T02:03:04Z"),
    )
    .await;
    insert_terms_acceptance(&database, equal_timestamp_user, "2026-08-11", legacy_at).await;
    insert_terms_acceptance(&database, newer_acceptance_user, "2026-08-11", later_at).await;

    Migrator::up(&database, Some(1))
        .await
        .expect("миграция переноса должна примениться");

    let equal_timestamp_rows = terms_acceptances(&database, equal_timestamp_user).await;
    assert_eq!(equal_timestamp_rows.len(), 3);
    assert!(equal_timestamp_rows.iter().any(|row| {
        row.version.as_deref() == Some("2026-08-11") && row.accepted_at == legacy_at
    }));
    assert!(equal_timestamp_rows.iter().any(|row| {
        row.version.is_none()
            && row.source == "legacy_users.accepted_terms_at"
            && row.accepted_at == legacy_at
    }));

    let newer_rows = terms_acceptances(&database, newer_acceptance_user).await;
    assert_eq!(newer_rows.len(), 2);
    assert!(newer_rows.iter().any(|row| {
        row.version.is_none()
            && row.source == "legacy_users.accepted_terms_at"
            && row.accepted_at == legacy_at
    }));
    assert!(newer_rows.iter().any(|row| {
        row.version.as_deref() == Some("2026-08-11") && row.accepted_at == later_at
    }));

    let missing_rows = terms_acceptances(&database, no_acceptance_user).await;
    assert_eq!(missing_rows.len(), 1);
    assert_eq!(missing_rows[0].version, None);
    assert_eq!(missing_rows[0].accepted_at, legacy_at);

    let legacy_duplicate = database
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO legal_acceptances (id, user_id, document_kind, document_version, acceptance_source, accepted_at) VALUES ($1, $2, 'terms', NULL, 'legacy_users.accepted_terms_at', $3)",
            [
                Uuid::new_v4().into(),
                no_acceptance_user.into(),
                legacy_at.into(),
            ],
        ))
        .await;
    assert!(legacy_duplicate.is_err());

    let legacy_column = database
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT column_name FROM information_schema.columns WHERE table_name = 'users' AND column_name = 'accepted_terms_at'",
            [],
        ))
        .await
        .expect("схема users должна читаться");
    assert!(legacy_column.is_none());

    Migrator::down(&database, Some(1))
        .await
        .expect("откат должен восстановить исходный столбец");
    assert_eq!(
        accepted_terms_at(&database, equal_timestamp_user).await,
        legacy_at
    );
    assert_eq!(
        accepted_terms_at(&database, newer_acceptance_user).await,
        legacy_at
    );
    assert_eq!(
        accepted_terms_at(&database, no_acceptance_user).await,
        legacy_at
    );
}

async fn insert_user(
    database: &sea_orm::DatabaseConnection,
    id: Uuid,
    nickname: &str,
    accepted_at: DateTime<Utc>,
) {
    database
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO users (id, nickname, email, email_normalized, password_hash, registered_at, nickname_updated_at, accepted_terms_at, updated_at) VALUES ($1, $2, $3, $3, 'test-hash', $4, $4, $5, $4)",
            [
                id.into(),
                nickname.to_owned().into(),
                format!("{nickname}@example.test").into(),
                accepted_at.into(),
                accepted_at.into(),
            ],
        ))
        .await
        .expect("тестовый пользователь должен создаться");
}

async fn insert_terms_acceptance(
    database: &sea_orm::DatabaseConnection,
    user_id: Uuid,
    version: &str,
    accepted_at: DateTime<Utc>,
) {
    database
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO legal_acceptances (id, user_id, document_kind, document_version, acceptance_source, accepted_at) VALUES ($1, $2, 'terms', $3, 'registration', $4)",
            [
                Uuid::new_v4().into(),
                user_id.into(),
                version.to_owned().into(),
                accepted_at.into(),
            ],
        ))
        .await
        .expect("тестовое подтверждение должно сохраниться");
}

async fn terms_acceptances(
    database: &sea_orm::DatabaseConnection,
    user_id: Uuid,
) -> Vec<AcceptanceRow> {
    database
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT document_version, acceptance_source, accepted_at FROM legal_acceptances WHERE user_id = $1 AND document_kind = 'terms' ORDER BY accepted_at",
            [user_id.into()],
        ))
        .await
        .expect("подтверждения должны читаться")
        .into_iter()
        .map(|row| AcceptanceRow {
            version: row.try_get("", "document_version").expect("версия должна быть допустимой"),
            source: row.try_get("", "acceptance_source").expect("источник должен быть строкой"),
            accepted_at: row.try_get("", "accepted_at").expect("дата должна быть допустимой"),
        })
        .collect()
}

async fn accepted_terms_at(database: &sea_orm::DatabaseConnection, user_id: Uuid) -> DateTime<Utc> {
    database
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT accepted_terms_at FROM users WHERE id = $1",
            [user_id.into()],
        ))
        .await
        .expect("исходная дата должна читаться")
        .expect("пользователь должен оставаться в базе")
        .try_get("", "accepted_terms_at")
        .expect("дата должна быть корректной")
}

fn timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.with_timezone(&Utc))
        .expect("тестовая дата должна быть корректной")
}

struct AcceptanceRow {
    version: Option<String>,
    source: String,
    accepted_at: DateTime<Utc>,
}
