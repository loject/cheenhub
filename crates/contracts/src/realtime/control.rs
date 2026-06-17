//! Контракты realtime-модуля управления.

use serde::{Deserialize, Serialize};

use crate::rest::AuthUser;

/// Виды сообщений модуля управления.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlKind {
    /// Аутентифицировать только что открытую realtime-сессию.
    Authenticate,
    /// Подтвердить успешную аутентификацию realtime.
    Authenticated,
    /// Временный диагностический запрос модуля управления.
    ControlText,
    /// Временный диагностический ответ модуля управления.
    ControlAck,
    /// Отклонить realtime-запрос или сессию.
    Rejected,
}

/// Полезная нагрузка запроса для аутентификации realtime-сессии.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authenticate {
    /// Короткоживущий access JWT.
    pub access_token: String,
}

/// Полезная нагрузка ответа после успешной аутентификации realtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authenticated {
    /// Аутентифицированный пользователь, привязанный к realtime-сессии.
    pub user: AuthUser,
}

/// Полезная нагрузка временного надежного диагностического запроса.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlText {
    /// Текст диагностического сообщения.
    pub body: String,
}

/// Полезная нагрузка временного надежного диагностического ответа.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlAck {
    /// Текст диагностического ответа.
    pub body: String,
}

/// Стабильный код отклонения realtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionCode {
    /// Учетные данные или состояние сессии недействительны.
    Unauthorized,
    /// Неверны форма сообщения, модуль, вид или полезная нагрузка.
    BadRequest,
    /// Запрошенный модуль не поддерживается.
    UnsupportedModule,
    /// Запрошенный вид сообщения не поддерживается модулем.
    UnsupportedMessage,
    /// Неожиданная ошибка сервера.
    InternalError,
}

/// Полезная нагрузка отклонения для ошибок протокола realtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rejected {
    /// Стабильный машинно-читаемый код отклонения.
    pub code: RejectionCode,
    /// Человекочитаемое сообщение об отклонении.
    pub message: String,
}
