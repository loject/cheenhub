//! Проверки мониторинга активности голосового чата для владельца хоста.

use std::sync::Arc;

use cheenhub_contracts::rest::RegisterRequest;
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::{activity, activity_history};
use crate::features::auth::application as auth_application;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::host_settings::activity_monitor::HostActivityMonitor;
use crate::features::host_settings::application::HostSettingsError;
use crate::features::host_settings::domain::VoiceActivitySample;
use crate::features::host_settings::infrastructure::InMemoryHostSettingsStore;
use crate::features::images::infrastructure::InMemoryImageStore;
use crate::features::push_notifications::application::PushNotifications;
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::social::infrastructure::InMemorySocialStore;
use crate::features::text_chat::infrastructure::{
    InMemoryChatAttachmentObjectStore, InMemoryTextChatStore,
};
use crate::features::voice_chat::infrastructure::{
    InMemoryDirectCallStore, InMemoryVoicePresenceStore, VoicePresence, VoicePresenceTargetKind,
};
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;

mod unavailable_store;

use unavailable_store::UnavailableHostSettingsStore;

fn state() -> AppState {
    AppState {
        auth_store: Arc::new(InMemoryAuthStore::default()),
        auth_mailer: Arc::new(crate::features::auth::email::tests::TestAuthMailer::default()),
        host_settings_store: Arc::new(InMemoryHostSettingsStore::default()),
        host_metrics: Arc::new(
            crate::features::host_settings::metrics_monitor::HostMetricsMonitor::disabled(),
        ),
        host_logs: Arc::new(crate::telemetry::HostLogHub::default()),
        server_store: Arc::new(InMemoryServerStore::default()),
        social_store: Arc::new(InMemorySocialStore::default()),
        text_chat_store: Arc::new(InMemoryTextChatStore::default()),
        chat_attachment_object_store: Arc::new(InMemoryChatAttachmentObjectStore::new(
            "activity-test-images",
        )),
        image_store: Arc::new(InMemoryImageStore::default()),
        push_notifications: Arc::new(PushNotifications::disabled(Arc::new(
            InMemoryAuthStore::default(),
        ))),
        image_processing_queue: Arc::new(tokio::sync::Semaphore::new(1)),
        voice_presence_store: Arc::new(InMemoryVoicePresenceStore::default()),
        direct_call_store: Arc::new(InMemoryDirectCallStore::default()),
        realtime_hub: Arc::new(RealtimeHub::default()),
        auth_keys: AuthKeys::generate_for_tests(),
        access_token_lifetime_minutes: 15,
        refresh_token_lifetime_days: 30,
        google_oauth_client_id: Some("activity-test-client".to_owned()),
        google_oauth_client_secret: Some("activity-test-secret".to_owned()),
        google_oauth_redirect_uri: Some(
            "http://localhost/api/auth/oauth/google/callback".to_owned(),
        ),
        typing_store: Arc::new(crate::features::typing::InMemoryTypingStore::default()),
        cheenhub_client_base_url: "http://localhost".to_owned(),
        cheenhub_api_base_url: "http://localhost/api".to_owned(),
        oauth_state_lifetime_minutes: 10,
        oauth_handoff_lifetime_minutes: 5,
        oauth_registration_lifetime_minutes: 15,
        password_reset_token_lifetime_minutes: 30,
    }
}

async fn register(
    state: &AppState,
    nickname: &str,
    email: &str,
) -> cheenhub_contracts::rest::AuthResponse {
    auth_application::register(
        state,
        RegisterRequest {
            nickname: nickname.to_owned(),
            email: email.to_owned(),
            password: "password123".to_owned(),
            accepts_terms: true,
            accepts_personal_data: true,
        },
    )
    .await
    .expect("registration should succeed")
}

async fn owner_state() -> (AppState, String) {
    let state = state();
    let auth = register(&state, "activity_owner", "activity-owner@example.com").await;
    let owner_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let state = AppState {
        host_settings_store: Arc::new(InMemoryHostSettingsStore::with_owner(owner_id)),
        ..state
    };
    (state, auth.access_token)
}

async fn join_voice_presence(state: &AppState, session_id: Uuid) -> Uuid {
    let realtime_stream_id = Uuid::new_v4();
    state
        .voice_presence_store
        .join(VoicePresence {
            realtime_stream_id,
            session_id,
            target_kind: VoicePresenceTargetKind::Server,
            server_id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            nickname: "activity_user".to_owned(),
            avatar_url: None,
            joined_at: Utc::now(),
        })
        .await;
    realtime_stream_id
}

fn sample_at(sampled_at: DateTime<Utc>, voice_connections: u32) -> VoiceActivitySample {
    VoiceActivitySample {
        id: Uuid::new_v4(),
        sampled_at,
        voice_connections,
        video_sources: 1,
    }
}
#[tokio::test]
async fn owner_sees_voice_connections_without_system_metrics() {
    let (state, token) = owner_state().await;
    join_voice_presence(&state, Uuid::new_v4()).await;
    join_voice_presence(&state, Uuid::new_v4()).await;

    let snapshot = activity(&state, &token)
        .await
        .expect("owner should read activity");

    assert_eq!(snapshot.voice_connections, 2);
    assert_eq!(snapshot.video_sources, 0);
    assert!(
        !state.host_metrics.snapshot().await.available,
        "системные метрики недоступны и не влияют на блок активности"
    );
}

#[tokio::test]
async fn regular_user_is_denied_activity_and_history() {
    let (state, _owner_token) = owner_state().await;
    let regular = register(&state, "activity_regular", "activity-regular@example.com").await;

    let denied = activity(&state, &regular.access_token).await;
    assert!(
        matches!(denied, Err(HostSettingsError::Forbidden(_))),
        "обычному пользователю текущая активность недоступна"
    );
    let denied = activity_history(&state, &regular.access_token, None).await;
    assert!(
        matches!(denied, Err(HostSettingsError::Forbidden(_))),
        "обычному пользователю история активности недоступна"
    );
}

#[tokio::test]
async fn history_returns_new_points_after_requested_time() {
    let (state, token) = owner_state().await;
    let store = state.host_settings_store.clone();
    let first_at = Utc::now() - Duration::minutes(2);
    store
        .insert_voice_activity_sample(sample_at(first_at, 1))
        .await
        .expect("insert first sample");
    store
        .insert_voice_activity_sample(sample_at(first_at + Duration::seconds(10), 5))
        .await
        .expect("insert second sample");

    let full = activity_history(&state, &token, None)
        .await
        .expect("owner should read history");
    assert!(full.available);
    assert_eq!(full.samples.len(), 2);
    assert_eq!(
        full.samples
            .iter()
            .map(|s| s.video_sources)
            .collect::<Vec<_>>(),
        vec![1, 1],
        "история содержит и видеоисточники"
    );

    // Клиент продолжает с отметки последней полученной точки, поэтому первая
    // уже загруженная точка повторно не приходит.
    let last_seen = full.samples.last().expect("history has samples");
    let incremental = activity_history(&state, &token, Some(last_seen.sampled_at_unix_ms))
        .await
        .expect("owner should read incremental history");
    assert_eq!(
        incremental
            .samples
            .iter()
            .map(|sample| sample.voice_connections)
            .collect::<Vec<_>>(),
        vec![5]
    );
}

#[tokio::test]
async fn unavailable_history_does_not_hide_current_activity() {
    let (state, token) = owner_state().await;
    join_voice_presence(&state, Uuid::new_v4()).await;
    let unavailable = Arc::new(UnavailableHostSettingsStore::new(
        state.host_settings_store.clone(),
    ));
    let state = AppState {
        host_settings_store: unavailable,
        ..state
    };

    let history = activity_history(&state, &token, None)
        .await
        .expect("history failure is reported as unavailable");
    assert!(!history.available);
    assert!(history.samples.is_empty());

    let snapshot = activity(&state, &token)
        .await
        .expect("current activity stays available when the database fails");
    assert_eq!(snapshot.voice_connections, 1);
}

#[tokio::test]
async fn monitor_stores_current_counters_including_zero() {
    let (state, _token) = owner_state().await;
    let store = state.host_settings_store.clone();
    let monitor = HostActivityMonitor::new(state.clone());
    let stream_id = join_voice_presence(&state, Uuid::new_v4()).await;

    assert!(
        monitor.collect_once().await,
        "первый тик сохраняет измерение"
    );

    let after_voice = store
        .load_voice_activity_samples(Utc::now() - Duration::minutes(1), Utc::now())
        .await
        .expect("load stored samples");
    assert_eq!(after_voice.last().expect("sample").voice_connections, 1);

    // Пустой хост тоже измеряется: нулевые значения сохраняются.
    state
        .voice_presence_store
        .leave_realtime_stream(&stream_id)
        .await;
    assert!(monitor.collect_once().await);

    let after_leave = store
        .load_voice_activity_samples(Utc::now() - Duration::minutes(1), Utc::now())
        .await
        .expect("load stored samples");
    assert_eq!(after_leave.last().expect("sample").voice_connections, 0);
}
