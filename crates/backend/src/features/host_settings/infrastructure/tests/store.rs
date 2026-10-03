//! Проверки хранилища настроек хоста.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::super::{HostSettingsStore, InMemoryHostSettingsStore, PostgresHostSettingsStore};
use crate::features::host_settings::activity_monitor::HISTORY_WINDOW;
use crate::features::host_settings::domain::{
    EmailTransport, GmailOAuthState, HostEmailSettings, VoiceActivitySample,
};

fn voice_sample(sampled_at: DateTime<Utc>, voice_connections: u32) -> VoiceActivitySample {
    VoiceActivitySample {
        id: Uuid::new_v4(),
        sampled_at,
        voice_connections,
        video_sources: voice_connections * 2,
    }
}

/// Проверяет общие правила истории для любой реализации хранилища.
async fn assert_history_contract(store: &dyn HostSettingsStore) {
    let now = Utc::now();
    let fresh = voice_sample(now - Duration::minutes(5), 3);
    let boundary = voice_sample(now - Duration::minutes(50), 7);
    store
        .insert_voice_activity_sample(fresh)
        .await
        .expect("insert fresh sample");
    store
        .insert_voice_activity_sample(boundary)
        .await
        .expect("insert boundary sample");

    // Полуинтервал (since, now]: клиент запрашивает только новые точки после прошлого ответа.
    let incremental = store
        .load_voice_activity_samples(now - Duration::minutes(10), now)
        .await
        .expect("load incremental history");
    assert_eq!(
        incremental,
        vec![fresh],
        "после границы приходит только новая точка"
    );

    // Полная история возвращается в хронологическом порядке.
    let full = store
        .load_voice_activity_samples(now - HISTORY_WINDOW, now)
        .await
        .expect("load full history");
    assert_eq!(full, vec![boundary, fresh]);
    assert_eq!(
        full.iter()
            .map(|sample| sample.video_sources)
            .collect::<Vec<_>>(),
        vec![14, 6],
        "история хранит оба показателя активности"
    );
}

#[tokio::test]
async fn in_memory_history_supports_incremental_reads_and_pruning() {
    let store = InMemoryHostSettingsStore::default();
    assert_history_contract(&store).await;

    let now = Utc::now();
    let outdated = voice_sample(now - HISTORY_WINDOW - Duration::minutes(5), 9);
    store
        .insert_voice_activity_sample(outdated)
        .await
        .expect("insert outdated sample");

    assert_eq!(
        store
            .delete_voice_activity_samples_before(now - HISTORY_WINDOW)
            .await
            .expect("prune history"),
        1
    );
    let remaining = store
        .load_voice_activity_samples(now - HISTORY_WINDOW * 2, now)
        .await
        .expect("load remaining history");
    assert!(
        !remaining.contains(&outdated),
        "записи старше 24 часов не возвращаются"
    );
}

/// Второй процесс читает историю, записанную до перезапуска backend.
#[tokio::test]
#[ignore = "Нужна изолированная cheenhub_activity_test с миграциями и CHEENHUB_TEST_DATABASE_URL"]
async fn postgres_history_survives_restart_and_prunes_old_samples() {
    let database_url = std::env::var("CHEENHUB_TEST_DATABASE_URL")
        .expect("set CHEENHUB_TEST_DATABASE_URL for the disposable database");
    let parsed = url::Url::parse(&database_url).expect("valid database URL");
    assert_eq!(
        parsed.path(),
        "/cheenhub_activity_test",
        "use only the disposable database"
    );
    let before_restart = PostgresHostSettingsStore::new(
        sea_orm::Database::connect(&database_url)
            .await
            .expect("connect test PostgreSQL"),
    );
    before_restart
        .delete_voice_activity_samples_before(Utc::now())
        .await
        .expect("clean history");
    let now = Utc::now();
    before_restart
        .insert_voice_activity_sample(voice_sample(now - Duration::minutes(1), 4))
        .await
        .expect("insert sample before restart");
    before_restart
        .insert_voice_activity_sample(voice_sample(now - Duration::minutes(2), 2))
        .await
        .expect("insert older sample");

    // Новый процесс читает ту же базу без переноса истории в память.
    let after_restart = PostgresHostSettingsStore::new(
        sea_orm::Database::connect(&database_url)
            .await
            .expect("connect second backend"),
    );
    let samples = after_restart
        .load_voice_activity_samples(now - HISTORY_WINDOW, Utc::now())
        .await
        .expect("load history after restart");
    assert_eq!(
        samples
            .iter()
            .map(|sample| sample.voice_connections)
            .collect::<Vec<_>>(),
        vec![2, 4],
        "история сохраняется через перезапуск и остаётся упорядоченной"
    );

    let outdated = voice_sample(Utc::now() - HISTORY_WINDOW - Duration::minutes(1), 5);
    after_restart
        .insert_voice_activity_sample(outdated)
        .await
        .expect("insert outdated sample");
    assert_eq!(
        after_restart
            .delete_voice_activity_samples_before(Utc::now() - HISTORY_WINDOW)
            .await
            .expect("prune postgres history"),
        1
    );
    let remaining = after_restart
        .load_voice_activity_samples(Utc::now() - HISTORY_WINDOW * 2, Utc::now())
        .await
        .expect("load remaining postgres history");
    assert!(!remaining.contains(&outdated));
}

#[tokio::test]
async fn distinguishes_host_owner_from_regular_user() {
    let owner_id = Uuid::new_v4();
    let regular_id = Uuid::new_v4();
    let store = InMemoryHostSettingsStore::with_owner(owner_id);

    assert!(store.is_host_owner(owner_id).await.expect("owner lookup"));
    assert!(!store.is_host_owner(regular_id).await.expect("owner lookup"));
}

#[tokio::test]
async fn gmail_oauth_state_is_consumed_only_once() {
    let owner_id = Uuid::new_v4();
    let store = InMemoryHostSettingsStore::with_owner(owner_id);
    let now = Utc::now();
    store
        .insert_gmail_oauth_state(GmailOAuthState {
            id: Uuid::new_v4(),
            state_hash: "hash".to_owned(),
            user_id: owner_id,
            created_at: now,
            expires_at: now + Duration::minutes(10),
        })
        .await
        .expect("state insert");

    assert_eq!(
        store
            .consume_gmail_oauth_state("hash", now)
            .await
            .expect("first consume"),
        Some(owner_id)
    );
    assert_eq!(
        store
            .consume_gmail_oauth_state("hash", now)
            .await
            .expect("second consume"),
        None
    );
}

#[tokio::test]
async fn email_transport_change_is_visible_without_recreating_store() {
    let owner_id = Uuid::new_v4();
    let store = InMemoryHostSettingsStore::with_owner(owner_id);
    assert_eq!(
        store
            .load_email_settings()
            .await
            .expect("initial settings")
            .transport,
        EmailTransport::Smtp
    );

    store
        .save_email_settings(
            HostEmailSettings {
                transport: EmailTransport::GmailApi,
                ..HostEmailSettings::default()
            },
            owner_id,
            Utc::now(),
        )
        .await
        .expect("settings update");

    assert_eq!(
        store
            .load_email_settings()
            .await
            .expect("updated settings")
            .transport,
        EmailTransport::GmailApi
    );
}
