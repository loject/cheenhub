//! Общие контракты ошибок REST.

use serde::{Deserialize, Serialize};

/// Структурированный ответ с ошибкой, возвращаемый REST-эндпоинтами.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    /// Стабильный машинно-читаемый код ошибки.
    pub code: String,
    /// Сообщение об ошибке для пользователя.
    pub message: String,
}
