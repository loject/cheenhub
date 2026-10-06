//! Проверки выдачи и отзыва прав владельца хоста.

use std::sync::Arc;

use cheenhub_contracts::rest::{GrantHostOwnerRequest, RegisterRequest};
use uuid::Uuid;

use super::{grant_owner, list_owners, revoke_owner};
use crate::features::auth::application as auth_application;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::host_settings::application::HostSettingsError;
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

fn base_state() -> AppState {
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
            "owners-test-images",
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
        google_oauth_client_id: None,
        google_oauth_client_secret: None,
        google_oauth_redirect_uri: None,
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

fn user_id(auth: &cheenhub_contracts::rest::AuthResponse) -> Uuid {
    Uuid::parse_str(&auth.user.id).expect("user id should be uuid")
}

/// Хост с одним владельцем и вторым зарегистрированным пользователем.
async fn host_with_owner_and_guest() -> (AppState, cheenhub_contracts::rest::AuthResponse, Uuid) {
    let state = base_state();
    let owner = register(&state, "owners_owner", "owner@example.com").await;
    let owner_id = user_id(&owner);
    let state = AppState {
        host_settings_store: Arc::new(InMemoryHostSettingsStore::with_owner(owner_id)),
        ..state
    };
    let guest = register(&state, "owners_guest", "guest@example.com").await;

    (state, owner, user_id(&guest))
}

/// Выдаёт новому пользователю права владельца и возвращает его идентификатор.
async fn granted_owner(
    state: &AppState,
    owner: &cheenhub_contracts::rest::AuthResponse,
    nickname: &str,
    email: &str,
) -> Uuid {
    let user_id = user_id(&register(state, nickname, email).await);
    grant_owner(
        state,
        &owner.access_token,
        GrantHostOwnerRequest {
            user: user_id.to_string(),
        },
    )
    .await
    .expect("grant should succeed");

    user_id
}

#[tokio::test]
async fn grants_owner_rights_by_email_and_lists_both_owners() {
    let (state, owner, _) = host_with_owner_and_guest().await;

    let response = grant_owner(
        &state,
        &owner.access_token,
        GrantHostOwnerRequest {
            user: "guest@example.com".to_owned(),
        },
    )
    .await
    .expect("grant should succeed");

    let owners: Vec<_> = response
        .owners
        .iter()
        .map(|entry| (entry.nickname.as_str(), entry.is_current_user))
        .collect();
    assert_eq!(
        owners,
        vec![("owners_owner", true), ("owners_guest", false)],
        "после выдачи на хосте два владельца"
    );
    assert_eq!(
        response.owners[1].granted_by_nickname.as_deref(),
        Some("owners_owner"),
        "владелец, выдавший права, сохраняется в записи"
    );
}

#[tokio::test]
async fn granted_owner_gains_access_to_host_settings() {
    let (state, owner, _) = host_with_owner_and_guest().await;

    granted_owner(&state, &owner, "owners_second", "second@example.com").await;

    assert!(
        list_owners(&state, &owner.access_token)
            .await
            .expect("owners should list")
            .owners
            .len()
            == 2,
        "выданные права дают доступ к настройкам хоста"
    );
}

#[tokio::test]
async fn new_owner_loses_access_after_revocation() {
    let (state, owner, _) = host_with_owner_and_guest().await;
    let granted_id = granted_owner(&state, &owner, "owners_second", "second@example.com").await;

    revoke_owner(&state, &owner.access_token, granted_id)
        .await
        .expect("revoke should succeed");

    let remaining = list_owners(&state, &owner.access_token)
        .await
        .expect("remaining owners should list")
        .owners;
    assert_eq!(
        remaining.len(),
        1,
        "после отзыва остался только первоначальный владелец"
    );
    assert!(
        !state
            .host_settings_store
            .is_host_owner(granted_id)
            .await
            .expect("owner lookup"),
        "отозванный владелец теряет права"
    );
}
#[tokio::test]
async fn revoking_the_last_owner_is_rejected() {
    let (state, owner, _) = host_with_owner_and_guest().await;
    let owner_id = user_id(&owner);

    let error = revoke_owner(&state, &owner.access_token, owner_id)
        .await
        .expect_err("последнего владельца отозвать нельзя");

    assert!(
        matches!(error, HostSettingsError::BadRequest(_)),
        "ожидался отказ с понятным сообщением"
    );
    assert!(
        state
            .host_settings_store
            .is_host_owner(owner_id)
            .await
            .expect("owner lookup"),
        "единственный владелец сохраняет права после отказа"
    );
}

#[tokio::test]
async fn revoking_owner_rights_requires_owner_access() {
    let (state, owner, _) = host_with_owner_and_guest().await;
    let granted_id = granted_owner(&state, &owner, "owners_second", "second@example.com").await;
    let outsider = register(&state, "owners_outsider", "outsider@example.com").await;

    let error = revoke_owner(&state, &outsider.access_token, granted_id)
        .await
        .expect_err("обычный пользователь не может отзывать права");

    assert!(
        matches!(error, HostSettingsError::Forbidden(_)),
        "ожидался запрет по правам доступа"
    );
}

#[tokio::test]
async fn granting_the_same_user_twice_is_rejected() {
    let (state, owner, _) = host_with_owner_and_guest().await;
    granted_owner(&state, &owner, "owners_second", "second@example.com").await;

    let error = grant_owner(
        &state,
        &owner.access_token,
        GrantHostOwnerRequest {
            user: "second@example.com".to_owned(),
        },
    )
    .await
    .expect_err("повторная выдача не должна дублировать владельца");

    assert!(matches!(error, HostSettingsError::BadRequest(_)));
    let owners = list_owners(&state, &owner.access_token)
        .await
        .expect("owners should list")
        .owners;
    assert_eq!(owners.len(), 2, "владелец не продублирован");
}

#[tokio::test]
async fn granting_rights_to_unknown_user_is_rejected() {
    let (state, owner, _) = host_with_owner_and_guest().await;

    let error = grant_owner(
        &state,
        &owner.access_token,
        GrantHostOwnerRequest {
            user: "nobody@example.com".to_owned(),
        },
    )
    .await
    .expect_err("неизвестный пользователь не получает права");

    assert!(matches!(error, HostSettingsError::BadRequest(_)));
}

#[tokio::test]
async fn owner_rights_cannot_be_granted_to_deleted_account() {
    let (state, owner, guest_id) = host_with_owner_and_guest().await;
    let now = chrono::Utc::now();
    state
        .auth_store
        .begin_account_deletion(
            &guest_id,
            "hash".to_owned(),
            now,
            now + chrono::Duration::days(30),
        )
        .await
        .expect("удаление аккаунта должно начаться");

    let error = grant_owner(
        &state,
        &owner.access_token,
        GrantHostOwnerRequest {
            user: guest_id.to_string(),
        },
    )
    .await
    .expect_err("удалённому аккаунту права не выдаются");

    assert!(matches!(error, HostSettingsError::BadRequest(_)));
}

#[tokio::test]
async fn granting_owner_waits_for_target_account_deletion_lifecycle() {
    use std::future::Future;
    use std::task::Poll;

    let (state, owner, guest_id) = host_with_owner_and_guest().await;
    let guard = state
        .auth_store
        .lock_account_lifecycle(&guest_id)
        .await
        .unwrap();
    let mut granting = std::pin::pin!(grant_owner(
        &state,
        &owner.access_token,
        GrantHostOwnerRequest {
            user: guest_id.to_string()
        },
    ));

    // In-memory зависимости готовы сразу; незавершённый poll означает ожидание
    // удерживаемой lifecycle-блокировки, без гонки scheduler и wall-clock таймера.
    let first_poll = std::future::poll_fn(|cx| Poll::Ready(granting.as_mut().poll(cx))).await;
    assert!(
        first_poll.is_pending(),
        "выдача обязана дождаться lifecycle удаления"
    );
    let now = chrono::Utc::now();
    state
        .auth_store
        .begin_account_deletion(
            &guest_id,
            "hash".to_owned(),
            now,
            now + chrono::Duration::days(30),
        )
        .await
        .unwrap();
    drop(guard);
    let result = granting.await;

    assert!(matches!(result, Err(HostSettingsError::BadRequest(_))));
    assert!(
        !state
            .host_settings_store
            .is_host_owner(guest_id)
            .await
            .unwrap()
    );
}
