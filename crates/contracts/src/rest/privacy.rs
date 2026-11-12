//! Контракт настроек приватности и списка блокировок.

use serde::{Deserialize, Serialize};

/// Кто может начинать переписку с пользователем.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectMessageAudience {
    /// Любой пользователь, если не действует блокировка.
    #[default]
    Everyone,
    /// Только друзья и собеседники, которым владелец написал после ограничения.
    Friends,
}

/// Кто может инициировать личный звонок или приглашение в голосовую комнату.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncomingContactAudience {
    /// Любой пользователь, если не действует блокировка.
    Everyone,
    /// Только друзья.
    #[default]
    Friends,
    /// Никто, включая друзей.
    Nobody,
}

/// Три независимые настройки входящих обращений пользователя.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct UserPrivacySettings {
    /// Правило начала личной переписки.
    pub direct_messages: DirectMessageAudience,
    /// Правило входящих личных звонков.
    pub direct_calls: IncomingContactAudience,
    /// Правило приглашений в голосовые комнаты.
    pub voice_room_invites: IncomingContactAudience,
}

/// Пользователь, которого владелец добавил в список блокировок.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedUserSummary {
    /// UUID заблокированного пользователя в строковом формате.
    pub user_id: String,
    /// Публичный никнейм пользователя.
    pub nickname: String,
    /// URL аватара, если он установлен.
    pub avatar_url: Option<String>,
    /// Время блокировки в формате RFC3339.
    pub blocked_at: String,
}

/// Список активных блокировок текущего пользователя.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListBlockedUsersResponse {
    /// Пользователи, заблокированные владельцем списка.
    pub users: Vec<BlockedUserSummary>,
}
