//! Контракты браузерного Google OAuth для настольного приложения.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Запущенная попытка Google OAuth, связанная с исходным приложением.
#[derive(Clone, Serialize, Deserialize)]
pub struct GoogleDesktopAuthStartResponse {
    /// Идентификатор попытки входа.
    pub attempt_id: Uuid,
    /// Секрет опроса; после готовности используется как `handoff_code`.
    pub poll_secret: String,
    /// Адрес Google, открываемый в системном браузере.
    pub authorization_url: String,
    /// Время жизни попытки в секундах.
    pub expires_in_seconds: u64,
    /// Минимальный рекомендуемый интервал опроса в секундах.
    pub poll_interval_seconds: u64,
}

/// Подтверждение владения попыткой для опроса и отмены.
#[derive(Clone, Serialize, Deserialize)]
pub struct GoogleDesktopAuthRequest {
    /// Идентификатор попытки входа.
    pub attempt_id: Uuid,
    /// Секрет исходного экземпляра приложения.
    pub poll_secret: String,
}

/// Состояние входа; секреты и данные Google в ответ не включаются.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GoogleDesktopAuthPollResponse {
    /// Ожидается подтверждение Google в браузере.
    Pending,
    /// Приложение может завершить вход с исходным `poll_secret`.
    Ready,
    /// Попытка отменена приложением.
    Cancelled,
    /// Время ожидания истекло.
    Expired,
    /// Провайдер или сервер не смог завершить вход.
    Failed {
        /// Безопасное сообщение для пользователя.
        message: String,
    },
}
