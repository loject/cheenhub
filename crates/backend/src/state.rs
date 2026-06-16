//! Общее состояние приложения бэкенда.

use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::features::auth::email::AuthMailer;
use crate::features::auth::infrastructure::AuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::images::infrastructure::ImageStore;
use crate::features::servers::infrastructure::ServerStore;
use crate::features::text_chat::infrastructure::{ChatAttachmentObjectStore, TextChatStore};
use crate::features::voice_chat::infrastructure::InMemoryVoicePresenceStore;
use crate::realtime::hub::RealtimeHub;

/// Общее состояние приложения бэкенда.
#[derive(Clone)]
pub(crate) struct AppState {
    /// Бэкенд хранения аутентификации.
    pub(crate) auth_store: Arc<dyn AuthStore>,
    /// Отправитель писем аутентификации.
    pub(crate) auth_mailer: Arc<dyn AuthMailer>,
    /// Бэкенд хранения серверов.
    pub(crate) server_store: Arc<dyn ServerStore>,
    /// Бэкенд хранения текстового чата.
    pub(crate) text_chat_store: Arc<dyn TextChatStore>,
    /// Бэкенд объектного хранения вложений текстового чата.
    pub(crate) chat_attachment_object_store: Arc<dyn ChatAttachmentObjectStore>,
    /// Бэкенд хранения изображений.
    pub(crate) image_store: Arc<dyn ImageStore>,
    /// Очередь на уровне процесса, ограничивающая параллельность обработки изображений.
    pub(crate) image_processing_queue: Arc<Semaphore>,
    /// Активное присутствие в голосовых комнатах.
    pub(crate) voice_presence_store: Arc<InMemoryVoicePresenceStore>,
    /// Общий реестр потоков realtime и хаб вещания.
    pub(crate) realtime_hub: Arc<RealtimeHub>,
    /// Ключи подписи Access JWT.
    pub(crate) auth_keys: AuthKeys,
    /// Время жизни Access JWT в минутах.
    pub(crate) access_token_lifetime_minutes: i64,
    /// Время жизни Refresh токена в днях.
    pub(crate) refresh_token_lifetime_days: i64,
    /// ID клиента Google OAuth.
    pub(crate) google_oauth_client_id: Option<String>,
    /// Секрет клиента Google OAuth.
    pub(crate) google_oauth_client_secret: Option<String>,
    /// URI перенаправления Google OAuth, зарегистрированный для этого бэкенда.
    pub(crate) google_oauth_redirect_uri: Option<String>,
    /// Базовый URL клиента браузера после обратных вызовов OAuth.
    pub(crate) cheenhub_client_base_url: String,
    /// Публичный базовый URL REST API для сгенерированных ссылок на ресурсы.
    pub(crate) cheenhub_api_base_url: String,
    /// Время жизни состояния OAuth в минутах.
    pub(crate) oauth_state_lifetime_minutes: i64,
    /// Время жизни передачи OAuth в минутах.
    pub(crate) oauth_handoff_lifetime_minutes: i64,
    /// Время жизни намерения регистрации OAuth в минутах.
    pub(crate) oauth_registration_lifetime_minutes: i64,
    /// Время жизни токена сброса пароля в минутах.
    pub(crate) password_reset_token_lifetime_minutes: i64,
}
