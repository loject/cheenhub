//! REST-контракты регистрации устройств для системных push-уведомлений.

use serde::{Deserialize, Serialize};

/// Платформа, предоставившая системный push-токен.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PushPlatform {
    /// Android-устройство с токеном Firebase Cloud Messaging.
    Android,
}

/// Запрос регистрации или обновления push-установки текущей auth-сессии.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpsertPushInstallationRequest {
    /// Платформа установки.
    pub platform: PushPlatform,
    /// Непрозрачный токен доставки, выданный push-провайдером.
    pub token: String,
}
