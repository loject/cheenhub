//! Сценарии регистрации и доставки системных push-уведомлений.

use std::sync::Arc;
use std::time::Duration as StdDuration;

use cheenhub_contracts::rest::{PushPlatform, UpsertPushInstallationRequest};
use chrono::Duration;
use chrono::Utc;
use uuid::Uuid;

use crate::features::auth::application::require_current_user;
use crate::features::auth::error::AuthError;
use crate::features::auth::infrastructure::AuthStore;
use crate::features::push_notifications::domain::DirectMessagePush;
use crate::features::push_notifications::error::PushError;
use crate::features::push_notifications::fcm::{FcmClient, FcmSendError};
use crate::features::push_notifications::infrastructure::PostgresPushStore;
use crate::state::AppState;

const MAX_DELIVERY_ATTEMPTS: i32 = 8;
const DELIVERY_TTL_HOURS: i64 = 24;

/// Координатор постоянной очереди и платформенного FCM-адаптера.
#[derive(Clone)]
pub(crate) struct PushNotifications {
    store: Option<PostgresPushStore>,
    fcm: Option<FcmClient>,
    auth_store: Arc<dyn AuthStore>,
}

impl PushNotifications {
    /// Создаёт отключённый координатор для in-memory окружений и тестов.
    pub(crate) fn disabled(auth_store: Arc<dyn AuthStore>) -> Self {
        Self {
            store: None,
            fcm: None,
            auth_store,
        }
    }

    /// Создаёт координатор с Postgres-хранилищем и необязательной FCM-доставкой.
    pub(crate) fn postgres(
        store: PostgresPushStore,
        fcm: Option<FcmClient>,
        auth_store: Arc<dyn AuthStore>,
    ) -> Self {
        Self {
            store: Some(store),
            fcm,
            auth_store,
        }
    }

    /// Сообщает, можно ли запускать FCM worker.
    pub(crate) fn worker_enabled(&self) -> bool {
        self.store.is_some()
    }

    /// Ставит уведомление о личном сообщении в очередь активных auth-сессий адресата.
    pub(crate) async fn enqueue_direct_message(
        &self,
        recipient_user_id: Uuid,
        payload: DirectMessagePush,
    ) -> anyhow::Result<usize> {
        let Some(store) = self.store.as_ref() else {
            return Ok(0);
        };
        let message_id = Uuid::parse_str(&payload.message_id)?;
        let mut enqueued = 0;
        for installation in store.active_installations(recipient_user_id).await? {
            if !self
                .auth_store
                .session_is_active(&installation.session_id, Utc::now())
                .await?
            {
                continue;
            }
            store
                .enqueue(installation.id, message_id, &payload, Utc::now())
                .await?;
            enqueued += 1;
        }
        Ok(enqueued)
    }

    /// Выполняет постоянный цикл доставки готовых заданий.
    pub(crate) async fn run_delivery_worker(self: Arc<Self>) {
        let Some(store) = self.store.clone() else {
            return;
        };
        let fcm = self.fcm.clone();
        tracing::info!(
            provider = "fcm",
            delivery_enabled = fcm.is_some(),
            delivery_ttl_hours = DELIVERY_TTL_HOURS,
            "started push queue worker"
        );
        loop {
            match store
                .prune_expired(Utc::now() - Duration::hours(DELIVERY_TTL_HOURS))
                .await
            {
                Ok(0) => {}
                Ok(expired) => tracing::info!(expired, "removed expired push delivery jobs"),
                Err(error) => tracing::error!(%error, "failed to prune expired push delivery jobs"),
            }
            let Some(fcm) = fcm.as_ref() else {
                tokio::time::sleep(StdDuration::from_secs(30)).await;
                continue;
            };
            match store.due_deliveries(Utc::now()).await {
                Ok(deliveries) => {
                    for delivery in deliveries {
                        let active = self
                            .auth_store
                            .session_is_active(&delivery.session_id, Utc::now())
                            .await;
                        match active {
                            Ok(true) => {}
                            Ok(false) => {
                                if let Err(error) = store.complete(delivery.id).await {
                                    tracing::error!(%error, delivery_id = %delivery.id, "failed to remove delivery for inactive auth session");
                                }
                                continue;
                            }
                            Err(error) => {
                                tracing::error!(%error, delivery_id = %delivery.id, "failed to validate push auth session");
                                continue;
                            }
                        }

                        match fcm.send(&delivery.token, &delivery.payload).await {
                            Ok(()) => {
                                if let Err(error) = store.complete(delivery.id).await {
                                    tracing::error!(%error, delivery_id = %delivery.id, "failed to complete delivered push job");
                                } else {
                                    tracing::debug!(delivery_id = %delivery.id, message_id = %delivery.payload.message_id, "delivered direct message push");
                                }
                            }
                            Err(FcmSendError::Permanent(error)) => {
                                tracing::warn!(%error, delivery_id = %delivery.id, installation_id = %delivery.installation_id, "FCM permanently rejected push installation");
                                if let Err(deactivate_error) =
                                    store.deactivate(delivery.installation_id).await
                                {
                                    tracing::error!(%deactivate_error, installation_id = %delivery.installation_id, "failed to deactivate rejected push installation");
                                }
                                if let Err(complete_error) = store.complete(delivery.id).await {
                                    tracing::error!(%complete_error, delivery_id = %delivery.id, "failed to remove permanently rejected push job");
                                }
                            }
                            Err(FcmSendError::Retry(error)) => {
                                let attempts = delivery.attempts + 1;
                                if attempts >= MAX_DELIVERY_ATTEMPTS {
                                    tracing::error!(%error, delivery_id = %delivery.id, attempts, "push delivery exhausted retries");
                                    if let Err(complete_error) = store.complete(delivery.id).await {
                                        tracing::error!(%complete_error, delivery_id = %delivery.id, "failed to remove exhausted push job");
                                    }
                                } else {
                                    tracing::warn!(%error, delivery_id = %delivery.id, attempts, "push delivery scheduled for retry");
                                    if let Err(retry_error) =
                                        store.retry(delivery.id, attempts).await
                                    {
                                        tracing::error!(%retry_error, delivery_id = %delivery.id, "failed to reschedule push delivery");
                                    }
                                }
                            }
                        }
                    }
                }
                Err(error) => tracing::error!(%error, "failed to poll push delivery queue"),
            }
            tokio::time::sleep(StdDuration::from_secs(2)).await;
        }
    }
}

/// Регистрирует или обновляет установку текущей auth-сессии.
pub(crate) async fn upsert_installation(
    state: &AppState,
    access_token: &str,
    installation_id: String,
    request: UpsertPushInstallationRequest,
) -> Result<(), PushError> {
    let installation_id = parse_installation_id(&installation_id)?;
    let (user, session_id) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let token = request.token.trim();
    if token.is_empty() || token.len() > 4096 {
        return Err(PushError::BadRequest(
            "Некорректный push-токен устройства.".to_owned(),
        ));
    }
    let Some(store) = state.push_notifications.store.as_ref() else {
        return Err(PushError::Unavailable(
            "Регистрация push-уведомлений недоступна.".to_owned(),
        ));
    };
    let platform = match request.platform {
        PushPlatform::Android => "android",
    };
    store
        .upsert_installation(
            installation_id,
            user.id,
            session_id,
            platform,
            token.to_owned(),
            Utc::now(),
        )
        .await
        .map_err(PushError::Internal)?;
    tracing::info!(user_id = %user.id, session_id = %session_id, installation_id = %installation_id, platform, "registered push installation");
    Ok(())
}

/// Удаляет установку из текущей auth-сессии.
pub(crate) async fn delete_installation(
    state: &AppState,
    access_token: &str,
    installation_id: String,
) -> Result<(), PushError> {
    let installation_id = parse_installation_id(&installation_id)?;
    let (user, session_id) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let Some(store) = state.push_notifications.store.as_ref() else {
        return Err(PushError::Unavailable(
            "Регистрация push-уведомлений недоступна.".to_owned(),
        ));
    };
    if !store
        .delete_installation(installation_id, user.id, session_id)
        .await
        .map_err(PushError::Internal)?
    {
        return Err(PushError::NotFound("Push-установка не найдена.".to_owned()));
    }
    tracing::info!(user_id = %user.id, session_id = %session_id, installation_id = %installation_id, "deleted push installation");
    Ok(())
}

fn parse_installation_id(value: &str) -> Result<Uuid, PushError> {
    Uuid::parse_str(value)
        .map_err(|_| PushError::BadRequest("Некорректный идентификатор установки.".to_owned()))
}

fn map_auth_error(error: AuthError) -> PushError {
    match error {
        AuthError::Unauthorized(message) => PushError::Unauthorized(message),
        AuthError::RefreshRejected { message, .. }
        | AuthError::RefreshRotationInProgress(message) => PushError::Unauthorized(message),
        AuthError::Internal(error) => PushError::Internal(error),
        AuthError::BadRequest(message)
        | AuthError::Conflict(message)
        | AuthError::RateLimited(message) => PushError::Unauthorized(message),
        AuthError::Misconfigured { message, .. } => PushError::Unauthorized(message),
    }
}
