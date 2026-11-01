//! Realtime-контракты приглашений в серверные голосовые комнаты.

use serde::{Deserialize, Serialize};

/// Запрос списка участников сервера, которых можно пригласить в текущую комнату.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListVoiceRoomInviteCandidates {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор текущей голосовой комнаты.
    pub room_id: String,
}

/// Публичная информация о доступном для приглашения участнике сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceRoomInviteCandidate {
    /// Идентификатор пользователя.
    pub user_id: String,
    /// Отображаемое имя пользователя.
    pub nickname: String,
    /// Публичный URL аватара, если он задан.
    pub avatar_url: Option<String>,
}

/// Снимок доступных для приглашения участников сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceRoomInviteCandidatesSnapshot {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор текущей голосовой комнаты.
    pub room_id: String,
    /// Участники, которым можно отправить приглашение.
    pub candidates: Vec<VoiceRoomInviteCandidate>,
}

/// Запрос отправки приглашения в текущую серверную голосовую комнату.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendVoiceRoomInvite {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты, в которой находится отправитель.
    pub room_id: String,
    /// Идентификатор приглашаемого участника.
    pub recipient_user_id: String,
}

/// Решение получателя приглашения; игнорирование остаётся локальным действием.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceRoomInviteResponse {
    /// Принять приглашение.
    Accept,
    /// Отклонить приглашение.
    Decline,
}

/// Ответ получателя на приглашение.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RespondVoiceRoomInvite {
    /// UUID приглашения.
    pub invite_id: String,
    /// Решение получателя.
    pub response: VoiceRoomInviteResponse,
}

/// Отмена отправленного приглашения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelVoiceRoomInvite {
    /// UUID приглашения.
    pub invite_id: String,
}

/// Запрос актуальных приглашений для восстановления после переподключения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListVoiceRoomInvites;

/// Состояние приглашения в серверную голосовую комнату.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceRoomInviteStatus {
    /// Приглашение ожидает решения получателя.
    Pending,
    /// Получатель принял приглашение.
    Accepted,
    /// Получатель отклонил приглашение.
    Declined,
    /// Истекли 45 секунд ожидания ответа.
    Expired,
    /// Отправитель отменил приглашение.
    Cancelled,
}

/// Снимок одного приглашения с необходимыми для уведомления данными.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceRoomInviteSnapshot {
    /// Стабильный UUID приглашения.
    pub invite_id: String,
    /// Идентификатор сервера.
    pub server_id: String,
    /// Название сервера на момент приглашения.
    pub server_name: String,
    /// Идентификатор голосовой комнаты.
    pub room_id: String,
    /// Название комнаты на момент приглашения.
    pub room_name: String,
    /// Идентификатор приглашающего пользователя.
    pub caller_user_id: String,
    /// Имя приглашающего пользователя на момент приглашения.
    pub caller_nickname: String,
    /// Публичный URL аватара приглашающего, если он задан.
    pub caller_avatar_url: Option<String>,
    /// Идентификатор получателя приглашения.
    pub recipient_user_id: String,
    /// Имя получателя на момент отправки, если доступно.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient_nickname: Option<String>,
    /// Публичный URL аватара получателя, если он задан.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient_avatar_url: Option<String>,
    /// Текущее состояние приглашения.
    pub status: VoiceRoomInviteStatus,
    /// Время создания в формате RFC3339.
    pub created_at: String,
    /// Время истечения в формате RFC3339.
    pub expires_at: String,
    /// Время завершения в формате RFC3339; отсутствует у ожидающих приглашений.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<String>,
}

/// Адресное событие изменения приглашения для обеих сторон.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceRoomInviteLifecycleEvent {
    /// Актуальный снимок изменившегося приглашения.
    pub invite: VoiceRoomInviteSnapshot,
}

/// Актуальные приглашения пользователя.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceRoomInvitesSnapshot {
    /// Приглашения, доступные пользователю.
    pub invites: Vec<VoiceRoomInviteSnapshot>,
    /// Время сервера при формировании снимка для расчёта остатка ожидания без часов устройства.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_now: Option<String>,
}

#[cfg(test)]
mod tests;
