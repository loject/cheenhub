//! Контракты REST для аутентификации.

use serde::{Deserialize, Serialize};

/// Тело запроса для создания новой учетной записи по email и паролю.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterRequest {
    /// Публичный никнейм, видимый другим пользователям.
    pub nickname: String,
    /// Адрес email для входа.
    pub email: String,
    /// Обычный пароль, отправляемый по HTTPS.
    pub password: String,
    /// Принял ли пользователь обязательные правила.
    pub accepts_policies: bool,
}

/// Тело запроса для входа по email и паролю.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginRequest {
    /// Адрес email для входа.
    pub email: String,
    /// Обычный пароль, отправляемый по HTTPS.
    pub password: String,
}

/// Тело запроса для отправки письма сброса пароля.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordResetRequest {
    /// Адрес email, который должен получить ссылку для сброса пароля.
    pub email: String,
}

/// Тело запроса для завершения сброса пароля.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordResetConfirmRequest {
    /// Непрозрачный токен сброса из ссылки на сброс пароля.
    pub token: String,
    /// Новый обычный пароль, отправляемый по HTTPS.
    pub new_password: String,
}

/// Внешний OAuth-провайдер, поддерживаемый REST API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthProvider {
    /// Провайдер идентификации Google OAuth.
    Google,
}

/// Вид OAuth-потока, запрошенный клиентом.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthFlow {
    /// Войти или зарегистрироваться через внешний провайдер.
    Login,
    /// Привязать внешний провайдер к текущей аутентифицированной учетной записи.
    Link,
}

/// Тело запроса для запуска OAuth-авторизации.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthStartRequest {
    /// Вид OAuth-потока, запрошенный клиентом.
    pub flow: OAuthFlow,
}

/// Ответ, возвращаемый после подготовки OAuth-авторизации.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthStartResponse {
    /// URL авторизации провайдера, куда должен перейти браузер.
    pub authorization_url: String,
}

/// Тело запроса для завершения передачи OAuth callback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthCompleteRequest {
    /// Одноразовый код передачи, возвращаемый через callback URL фронтенда.
    pub handoff_code: String,
}

/// Ответ, возвращаемый при завершении передачи OAuth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OAuthCompleteResponse {
    /// OAuth создал аутентифицированную сессию CheenHub.
    Authenticated {
        /// Токены аутентификации и текущий пользователь.
        auth: AuthResponse,
    },
    /// Личность OAuth подтверждена, но новой учетной записи CheenHub нужен никнейм.
    RegistrationRequired {
        /// Одноразовый токен для завершения регистрации.
        registration_token: String,
        /// Подтвержденный адрес email от OAuth-провайдера.
        email: String,
        /// Отображаемое имя, возвращенное OAuth-провайдером.
        display_name: Option<String>,
    },
    /// OAuth привязал провайдера к текущей учетной записи.
    Linked {
        /// Привязанная внешняя учетная запись.
        account: LinkedAccount,
    },
}

/// Тело запроса для завершения OAuth-регистрации.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthRegistrationRequest {
    /// Одноразовый токен регистрации, возвращенный после завершения OAuth.
    pub registration_token: String,
    /// Публичный никнейм, выбранный для новой учетной записи CheenHub.
    pub nickname: String,
    /// Принял ли пользователь обязательные правила.
    pub accepts_policies: bool,
}

/// Тело запроса для обновления профиля текущего аутентифицированного пользователя.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCurrentUserRequest {
    /// Новый публичный никнейм, видимый другим пользователям.
    pub nickname: String,
}

/// Тело запроса для смены пароля текущего пользователя.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeCurrentUserPasswordRequest {
    /// Текущий обычный пароль, отправляемый по HTTPS.
    pub current_password: String,
    /// Новый обычный пароль, отправляемый по HTTPS.
    pub new_password: String,
    /// Повтор нового пароля для защиты от опечатки.
    pub new_password_confirmation: String,
}

/// Тело запроса для ротации refresh-токена.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshRequest {
    /// Непрозрачный refresh-токен, ранее выданный бэкендом.
    pub refresh_token: String,
}

/// Тело запроса для инвалидирования сессии.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogoutRequest {
    /// Непрозрачный refresh-токен, идентифицирующий сессию для инвалидирования.
    pub refresh_token: String,
}

/// Категория устройства, определенная по User-Agent сессии.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionDeviceKind {
    /// Браузер на настольном компьютере или ноутбуке.
    Desktop,
    /// Браузер телефона или web view мобильного приложения.
    Mobile,
    /// Браузер планшета или web view планшетного приложения.
    Tablet,
    /// Автоматизированный клиент, crawler или скриптоподобная среда.
    Bot,
    /// Неизвестный тип клиента.
    Unknown,
}

/// Разобранные данные User-Agent активной auth-сессии.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionClientInfo {
    /// Определенная категория устройства.
    pub device_kind: SessionDeviceKind,
    /// Человекочитаемое имя операционной системы.
    pub os_name: String,
    /// Человекочитаемое имя браузера или клиента.
    pub browser_name: String,
}

/// Активная auth-сессия, отображаемая в настройках безопасности учетной записи.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveSession {
    /// Стабильный идентификатор сессии.
    pub id: String,
    /// Разобранные данные User-Agent.
    pub client: SessionClientInfo,
    /// Последний нормализованный raw User-Agent, наблюдавшийся для сессии.
    pub user_agent: Option<String>,
    /// Временная метка RFC 3339 создания сессии.
    pub created_at: String,
    /// Временная метка RFC 3339 последнего обнаружения сессии.
    pub last_seen_at: String,
    /// Временная метка RFC 3339 истечения сессии, если ее не обновить.
    pub expires_at: String,
    /// Описывает ли эта строка access-токен, использованный для запроса.
    pub current: bool,
}

/// Ответ со списком активных auth-сессий текущего пользователя.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveSessionsResponse {
    /// Активные сессии, отсортированные по убыванию свежести активности.
    pub sessions: Vec<ActiveSession>,
}

/// Привязанная внешняя учетная запись, возвращаемая в настройки аккаунта.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedAccount {
    /// Внешний OAuth-провайдер.
    pub provider: OAuthProvider,
    /// Адрес email, сообщенный провайдером.
    pub email: String,
    /// Отображаемое имя, сообщенное провайдером.
    pub display_name: Option<String>,
    /// Временная метка RFC 3339 привязки провайдера.
    pub linked_at: String,
}

/// Ответ со списком внешних аккаунтов, привязанных к текущему пользователю.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedAccountsResponse {
    /// Привязанные внешние аккаунты.
    pub accounts: Vec<LinkedAccount>,
}

/// Тело запроса для отвязки аккаунта внешнего провайдера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnlinkProviderRequest {
    /// Внешний OAuth-провайдер для отвязки.
    pub provider: OAuthProvider,
}

/// Успешный ответ аутентификации.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthResponse {
    /// Короткоживущий access JWT, подписанный Ed25519.
    pub access_token: String,
    /// Долгоживущий непрозрачный refresh-токен.
    pub refresh_token: String,
    /// Профиль аутентифицированного пользователя.
    pub user: AuthUser,
}

/// Данные пользователя, возвращаемые auth-эндпоинтами.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthUser {
    /// Стабильный идентификатор пользователя.
    pub id: String,
    /// Публичный никнейм, видимый другим пользователям.
    pub nickname: String,
    /// Адрес email для входа.
    pub email: String,
    /// Временная метка регистрации в формате RFC 3339.
    pub registered_at: String,
    /// Есть ли у учетной записи локальный пароль.
    pub has_password: bool,
    /// Публичный URL аватара, если пользователь его настроил.
    pub avatar_url: Option<String>,
}
