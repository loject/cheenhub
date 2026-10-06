//! Проверки минимального уровня журналирования хоста.

use std::sync::Arc;

mod controlled_store;

static TEST_SETTINGS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

use cheenhub_contracts::rest::{HostLogLevel, RegisterRequest, UpdateHostLogSettingsRequest};
use uuid::Uuid;

use super::{restore, settings, update_settings};
use crate::features::auth::application as auth_application;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::host_settings::application::HostSettingsError;
use crate::features::host_settings::domain::{HostLogSettings, LogLevel};
use crate::features::host_settings::infrastructure::InMemoryHostSettingsStore;
use crate::features::images::infrastructure::InMemoryImageStore;
use crate::features::push_notifications::application::PushNotifications;
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::social::infrastructure::InMemorySocialStore;
use crate::features::text_chat::infrastructure::{
    InMemoryChatAttachmentObjectStore, InMemoryTextChatStore,
};
use crate::features::voice_chat::infrastructure::{
    InMemoryDirectCallStore, InMemoryVoicePresenceStore,
};
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;

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
            "log-settings-test-images",
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
        google_oauth_client_id: Some("log-settings-test-client".to_owned()),
        google_oauth_client_secret: Some("log-settings-test-secret".to_owned()),
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
    // Сценарии проверяют успешное применение уровня, поэтому процессный
    // фильтр инициализируется один раз без глобального подписчика.
    crate::telemetry::init_for_tests("info");

    let state = state();
    let auth = register(
        &state,
        "log_settings_owner",
        "log-settings-owner@example.com",
    )
    .await;
    let owner_id = Uuid::parse_str(&auth.user.id).expect("user id should be uuid");
    let state = AppState {
        host_settings_store: Arc::new(InMemoryHostSettingsStore::with_owner(owner_id)),
        ..state
    };
    (state, auth.access_token)
}

#[tokio::test]
async fn saved_level_is_visible_to_owner_and_marked_as_updated() {
    let _isolation = TEST_SETTINGS.lock().await;
    let (state, access_token) = owner_state().await;

    let saved = update_settings(
        &state,
        &access_token,
        UpdateHostLogSettingsRequest {
            min_level: Some(HostLogLevel::Warn),
        },
    )
    .await
    .expect("owner should update host log level");

    assert_eq!(saved.min_level, Some(HostLogLevel::Warn));
    assert!(saved.updated_at.is_some());

    let loaded = settings(&state, &access_token)
        .await
        .expect("owner should read host log level");
    assert_eq!(loaded.min_level, Some(HostLogLevel::Warn));
}
#[tokio::test]
async fn reset_returns_server_to_startup_filter() {
    let _isolation = TEST_SETTINGS.lock().await;
    let (state, access_token) = owner_state().await;
    update_settings(
        &state,
        &access_token,
        UpdateHostLogSettingsRequest {
            min_level: Some(HostLogLevel::Debug),
        },
    )
    .await
    .expect("owner should update host log level");

    let reset = update_settings(
        &state,
        &access_token,
        UpdateHostLogSettingsRequest { min_level: None },
    )
    .await
    .expect("owner should reset host log level");

    assert_eq!(reset.min_level, None);
    assert_eq!(crate::telemetry::current_filter_for_tests(), "info");
    assert!(reset.updated_at.is_some());

    let stored = state
        .host_settings_store
        .load_log_settings()
        .await
        .expect("stored level should load");
    assert_eq!(stored.min_level, None);
}

#[tokio::test]
async fn stored_level_is_read_back_after_startup_restore() {
    let _isolation = TEST_SETTINGS.lock().await;
    let state = state();
    state
        .host_settings_store
        .save_log_settings(
            HostLogSettings {
                min_level: Some(LogLevel::Error),
                updated_at: None,
            },
            Uuid::nil(),
            chrono::Utc::now(),
        )
        .await
        .expect("level should be seeded");

    crate::telemetry::init_for_tests("info");
    restore(&state).await;
    assert_eq!(crate::telemetry::current_filter_for_tests(), "error");

    let stored = state
        .host_settings_store
        .load_log_settings()
        .await
        .expect("stored level should load");
    assert_eq!(stored.min_level, Some(LogLevel::Error));
}

#[tokio::test]
async fn regular_user_cannot_change_host_log_level() {
    let _isolation = TEST_SETTINGS.lock().await;
    let state = state();
    let auth = register(&state, "log_settings_user", "log-settings-user@example.com").await;

    let denied = update_settings(
        &state,
        &auth.access_token,
        UpdateHostLogSettingsRequest {
            min_level: Some(HostLogLevel::Trace),
        },
    )
    .await;

    assert!(matches!(denied, Err(HostSettingsError::Forbidden(_))));
}

#[tokio::test]
async fn failed_save_restores_actual_runtime_filter() {
    let _isolation = TEST_SETTINGS.lock().await;
    let (mut state, access_token) = owner_state().await;
    crate::telemetry::set_log_level(Some("error")).unwrap();
    let store = Arc::new(controlled_store::ControlledStore::new(
        state.host_settings_store.clone(),
        true,
    ));
    state.host_settings_store = store;

    let result = update_settings(
        &state,
        &access_token,
        UpdateHostLogSettingsRequest {
            min_level: Some(HostLogLevel::Trace),
        },
    )
    .await;

    assert!(matches!(result, Err(HostSettingsError::Internal(_))));
    assert_eq!(crate::telemetry::current_filter_for_tests(), "error");
    assert_eq!(
        state
            .host_settings_store
            .load_log_settings()
            .await
            .unwrap()
            .min_level,
        None
    );
}

#[tokio::test]
async fn concurrent_updates_do_not_apply_second_filter_during_first_save() {
    let _isolation = TEST_SETTINGS.lock().await;
    let (mut state, access_token) = owner_state().await;
    let store = Arc::new(controlled_store::ControlledStore::new(
        state.host_settings_store.clone(),
        false,
    ));
    state.host_settings_store = store.clone();
    let first_state = state.clone();
    let first_token = access_token.clone();
    let first = tokio::spawn(async move {
        update_settings(
            &first_state,
            &first_token,
            UpdateHostLogSettingsRequest {
                min_level: Some(HostLogLevel::Error),
            },
        )
        .await
    });
    store.started.notified().await;
    let second = update_settings(
        &state,
        &access_token,
        UpdateHostLogSettingsRequest {
            min_level: Some(HostLogLevel::Trace),
        },
    );
    tokio::pin!(second);

    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), &mut second)
            .await
            .is_err()
    );
    assert_eq!(crate::telemetry::current_filter_for_tests(), "error");
    store.release.notify_one();
    first.await.unwrap().unwrap();
    second.await.unwrap();
    assert_eq!(
        state
            .host_settings_store
            .load_log_settings()
            .await
            .unwrap()
            .min_level,
        Some(LogLevel::Trace)
    );
    assert_eq!(crate::telemetry::current_filter_for_tests(), "trace");
}

#[tokio::test]
async fn cancelled_request_finishes_save_and_keeps_runtime_consistent() {
    let _isolation = TEST_SETTINGS.lock().await;
    let (mut state, access_token) = owner_state().await;
    let store = Arc::new(controlled_store::ControlledStore::new(
        state.host_settings_store.clone(),
        false,
    ));
    state.host_settings_store = store.clone();
    let request_state = state.clone();
    let request = tokio::spawn(async move {
        update_settings(
            &request_state,
            &access_token,
            UpdateHostLogSettingsRequest {
                min_level: Some(HostLogLevel::Error),
            },
        )
        .await
    });
    store.started.notified().await;

    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    store.release.notify_one();
    tokio::time::timeout(std::time::Duration::from_secs(1), store.finished.notified())
        .await
        .expect("save should finish after request cancellation");

    assert_eq!(
        state
            .host_settings_store
            .load_log_settings()
            .await
            .unwrap()
            .min_level,
        Some(LogLevel::Error)
    );
    assert_eq!(crate::telemetry::current_filter_for_tests(), "error");
}
