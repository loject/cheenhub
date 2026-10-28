//! REST-контракты одноразового входа desktop-клиента через веб-сессию CheenHub.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Попытка входа, созданная настольным приложением.
#[derive(Clone, Serialize, Deserialize)]
pub struct BrowserDesktopAuthStartResponse {
    /// Идентификатор попытки, который разрешено передавать браузеру.
    pub attempt_id: Uuid,
    /// Секрет, остающийся только в исходном настольном приложении.
    pub poll_secret: String,
    /// HTTPS-страница подтверждения в системном браузере.
    pub authorization_url: String,
    /// Короткий код для сверки в обоих интерфейсах.
    pub verification_code: String,
    /// Время действия попытки в секундах.
    pub expires_in_seconds: u64,
    /// Рекомендуемый интервал опроса в секундах.
    pub poll_interval_seconds: u64,
}

/// Доказательство владения попыткой исходным настольным приложением.
#[derive(Clone, Serialize, Deserialize)]
pub struct BrowserDesktopAuthRequest {
    /// Идентификатор попытки.
    pub attempt_id: Uuid,
    /// Случайный секрет, не раскрываемый браузеру.
    pub poll_secret: String,
}

/// Состояние попытки, не содержащее токенов и личных данных.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrowserDesktopAuthPollResponse {
    /// Браузер ещё не подтвердил вход.
    Pending,
    /// Исходное приложение может однократно получить сессию.
    Ready,
}

/// Публичная информация, достаточная для страницы подтверждения.
#[derive(Clone, Serialize, Deserialize)]
pub struct BrowserDesktopAuthDetailsResponse {
    /// Короткий код, который должен совпасть с кодом в приложении.
    pub verification_code: String,
}
