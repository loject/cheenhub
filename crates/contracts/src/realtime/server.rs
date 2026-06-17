//! Контракты realtime-модуля управления сервером.

use serde::{Deserialize, Serialize};

/// Виды сообщений realtime-модуля управления сервером.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerKind {
    /// Загрузить участников сервера.
    ListServerMembers,
    /// Ответ со списком участников сервера.
    ServerMemberList,
    /// Загрузить ссылки-приглашения сервера.
    ListServerInvites,
    /// Ответ со списком ссылок-приглашений.
    ServerInviteList,
    /// Отозвать одно приглашение сервера.
    RevokeServerInvite,
    /// Подтверждает, что приглашение было отозвано.
    ServerInviteRevoked,
    /// Исключить участника, вошедшего по приглашению.
    KickServerInviteMember,
    /// Подтверждает, что участник по приглашению был исключен.
    ServerInviteMemberKicked,
    /// Исключить активного участника сервера.
    KickServerMember,
    /// Подтверждает, что участник сервера был исключен.
    ServerMemberKicked,
    /// Загрузить роли сервера.
    ListServerRoles,
    /// Ответ со списком ролей сервера.
    ServerRoleList,
    /// Сохранить роли сервера.
    SaveServerRoles,
    /// Подтверждает, что роли сервера были сохранены.
    ServerRolesSaved,
    /// Назначить участнику сервера пользовательскую роль.
    AssignServerMemberRole,
    /// Подтверждает, что роль была назначена.
    ServerMemberRoleAssigned,
    /// Отозвать пользовательскую роль у участника сервера.
    RevokeServerMemberRole,
    /// Подтверждает, что роль была отозвана.
    ServerMemberRoleRevoked,
}

/// Полезная нагрузка запроса для загрузки участников сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListServerMembers {
    /// Идентификатор сервера.
    pub server_id: String,
}

/// Полезная нагрузка ответа со списком активных участников сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerMemberList {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Активные участники, видимые текущему администратору.
    pub members: Vec<ServerMemberEntry>,
}

/// Активный участник сервера, отображаемый в настройках.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerMemberEntry {
    /// Стабильный идентификатор пользователя.
    pub user_id: String,
    /// Текущий никнейм пользователя.
    pub nickname: String,
    /// Владеет ли этот участник сервером.
    pub is_owner: bool,
    /// Временная метка начала участия в формате RFC3339.
    pub joined_at: String,
    /// Ссылка-приглашение, использованная этим участником, если доступна.
    pub invite_code: Option<String>,
    /// Временная метка использования приглашения в формате RFC3339, если доступна.
    pub invite_used_at: Option<String>,
    /// Идентификаторы пользовательских ролей, которые сейчас назначены этому участнику.
    pub role_ids: Vec<String>,
}

/// Полезная нагрузка запроса для загрузки ссылок-приглашений сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListServerInvites {
    /// Идентификатор сервера.
    pub server_id: String,
}

/// Полезная нагрузка ответа со ссылками-приглашениями сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteList {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Ссылки-приглашения, доступные текущему администратору.
    pub invites: Vec<ServerInviteLink>,
}

/// Ссылка-приглашение сервера, отображаемая в настройках.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteLink {
    /// Стабильный код приглашения.
    pub code: String,
    /// Пользователь, создавший приглашение.
    pub author_user_id: String,
    /// Текущий никнейм создателя приглашения.
    pub author_nickname: String,
    /// Временная метка создания приглашения в формате RFC3339.
    pub created_at: String,
    /// Необязательная временная метка истечения приглашения в формате RFC3339.
    pub expires_at: Option<String>,
    /// Необязательный максимальный лимит использований приглашения.
    pub max_uses: Option<u32>,
    /// Количество успешных использований приглашения.
    pub uses: u32,
    /// Временная метка отзыва в формате RFC3339, когда приглашение отозвано.
    pub revoked_at: Option<String>,
    /// Участники, вошедшие по этому приглашению.
    pub joined_members: Vec<ServerInviteJoinedMember>,
}

/// Запись об участнике, вошедшем по приглашению.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteJoinedMember {
    /// Стабильный идентификатор пользователя.
    pub user_id: String,
    /// Текущий никнейм пользователя.
    pub nickname: String,
    /// Временная метка использования приглашения в формате RFC3339.
    pub joined_at: String,
    /// Является ли пользователь сейчас активным участником сервера.
    pub is_active_member: bool,
}

/// Полезная нагрузка запроса для отзыва одного приглашения сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevokeServerInvite {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Код приглашения для отзыва.
    pub code: String,
}

/// Полезная нагрузка ответа после отзыва одного приглашения сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteRevoked {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Отозванный код приглашения.
    pub code: String,
    /// Временная метка отзыва в формате RFC3339.
    pub revoked_at: String,
}

/// Полезная нагрузка запроса для исключения участника, вошедшего по приглашению.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KickServerInviteMember {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Код приглашения, использованный участником.
    pub invite_code: String,
    /// Идентификатор пользователя для исключения.
    pub user_id: String,
}

/// Полезная нагрузка ответа после исключения участника по приглашению.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteMemberKicked {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Код приглашения, использованный исключенным участником.
    pub invite_code: String,
    /// Идентификатор исключенного пользователя.
    pub user_id: String,
}

/// Полезная нагрузка запроса для исключения активного участника сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KickServerMember {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор пользователя для исключения.
    pub user_id: String,
    /// Необязательная длительность блокировки повторного входа в секундах.
    pub exclusion_duration_seconds: Option<u64>,
}

/// Полезная нагрузка ответа после исключения участника сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerMemberKicked {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор исключенного пользователя.
    pub user_id: String,
    /// Временная метка, до которой пользователь не может вернуться, в формате RFC3339.
    pub excluded_until: Option<String>,
}

/// Полезная нагрузка запроса для загрузки ролей сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListServerRoles {
    /// Идентификатор сервера.
    pub server_id: String,
}

/// Полезная нагрузка ответа со списком ролей сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerRoleList {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Роли, отсортированные от высшего приоритета к низшему.
    pub roles: Vec<ServerRoleEntry>,
}

/// Вид роли сервера.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerRoleKind {
    /// Обязательная роль владельца.
    Owner,
    /// Обязательная роль участника по умолчанию.
    Member,
    /// Роль, созданная пользователем.
    Custom,
}

/// Флаг права роли сервера.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerRolePermission {
    /// Разрешает создавать ссылки-приглашения сервера.
    CreateInviteLinks,
    /// Разрешает исключать участников из сервера.
    KickServerMembers,
    /// Разрешает управлять ролями сервера.
    ManageRoles,
    /// Разрешает исключать участников из голосовых комнат.
    KickVoiceMembers,
    /// Разрешает удалять любые сообщения в текстовых комнатах.
    DeleteMessages,
}

/// Краткая сводка роли сервера, встроенная в серверные ответы для проверки прав на клиенте.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerRoleSummary {
    /// Стабильный идентификатор роли.
    pub role_id: String,
    /// Вид роли (owner / member / custom).
    pub kind: ServerRoleKind,
    /// Права, предоставляемые этой ролью.
    pub permissions: Vec<ServerRolePermission>,
}

/// Роль сервера, отображаемая в настройках.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerRoleEntry {
    /// Стабильный идентификатор роли.
    pub role_id: String,
    /// Человекочитаемое имя роли.
    pub name: String,
    /// Цвет роли в hex.
    pub color: String,
    /// Число участников, у которых сейчас есть эта роль.
    pub members: u32,
    /// Обязательная ли эта роль и нельзя ли ее удалить.
    pub is_required: bool,
    /// Вид роли.
    pub kind: ServerRoleKind,
    /// Итоговые права роли.
    pub permissions: Vec<ServerRolePermission>,
}

/// Полезная нагрузка запроса для сохранения ролей сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveServerRoles {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Роли, отсортированные от высшего приоритета к низшему.
    pub roles: Vec<ServerRoleDraft>,
}

/// Черновик роли сервера, отправляемый из настроек.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerRoleDraft {
    /// Идентификатор существующей роли. Отсутствие id создает новую пользовательскую роль.
    pub role_id: Option<String>,
    /// Человекочитаемое имя роли.
    pub name: String,
    /// Цвет роли в hex.
    pub color: String,
    /// Вид роли.
    pub kind: ServerRoleKind,
    /// Включенные права роли.
    pub permissions: Vec<ServerRolePermission>,
}

/// Полезная нагрузка ответа после сохранения ролей сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerRolesSaved {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Сохраненные роли, отсортированные от высшего приоритета к низшему.
    pub roles: Vec<ServerRoleEntry>,
}

/// Полезная нагрузка запроса для назначения пользовательской роли участнику сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignServerMemberRole {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор целевого пользователя.
    pub user_id: String,
    /// Идентификатор пользовательской роли для назначения.
    pub role_id: String,
}

/// Полезная нагрузка ответа после назначения роли участнику сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerMemberRoleAssigned {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Пользователь, получивший роль.
    pub user_id: String,
    /// Назначенная роль.
    pub role_id: String,
}

/// Полезная нагрузка запроса для отзыва пользовательской роли у участника сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevokeServerMemberRole {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор целевого пользователя.
    pub user_id: String,
    /// Идентификатор пользовательской роли для отзыва.
    pub role_id: String,
}

/// Полезная нагрузка ответа после отзыва роли у участника сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerMemberRoleRevoked {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Пользователь, у которого отозвали роль.
    pub user_id: String,
    /// Отозванная роль.
    pub role_id: String,
}
