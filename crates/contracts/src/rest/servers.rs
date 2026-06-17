//! Контракты REST для серверов.

use serde::{Deserialize, Serialize};

use crate::realtime::ServerRoleSummary;

/// Тело запроса для создания нового сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateServerRequest {
    /// Человекочитаемое имя сервера.
    pub name: String,
}

/// Данные сервера, возвращаемые эндпоинтами серверов.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerSummary {
    /// Стабильный идентификатор сервера.
    pub id: String,
    /// Человекочитаемое имя сервера.
    pub name: String,
    /// Публичный URL аватара, если он настроен.
    pub avatar_url: Option<String>,
    /// Владеет ли текущий пользователь сервером.
    pub is_owner: bool,
    /// Является ли текущий пользователь активным участником сервера.
    pub is_member: bool,
    /// Все роли, определенные на этом сервере, с их правами.
    pub roles: Vec<ServerRoleSummary>,
    /// Идентификаторы ролей, которые сейчас назначены текущему пользователю на этом сервере.
    pub member_role_ids: Vec<String>,
}

/// Успешный ответ о создании сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateServerResponse {
    /// Созданный сервер.
    pub server: ServerSummary,
}

/// Тело запроса для обновления профиля сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateServerRequest {
    /// Человекочитаемое имя сервера.
    pub name: String,
}

/// Успешный ответ на обновление профиля сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateServerResponse {
    /// Обновленный сервер.
    pub server: ServerSummary,
}

/// Успешный ответ на обновление аватара сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateServerAvatarResponse {
    /// Обновленный сервер.
    pub server: ServerSummary,
}

/// Тип комнаты сервера, поддерживаемый MVP.
/// TODO: это не должен быть REST, а должен быть realtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerRoomKind {
    /// Комната только для текста.
    Text,
    /// Комната только для голоса.
    Voice,
    /// Комната с текстовыми и голосовыми возможностями.
    TextAndVoice,
}

/// Данные комнаты сервера, возвращаемые room-эндпоинтами.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerRoomSummary {
    /// Стабильный идентификатор комнаты.
    pub id: String,
    /// Человекочитаемое имя комнаты.
    pub name: String,
    /// Тип взаимодействия комнаты.
    pub kind: ServerRoomKind,
    /// Позиция комнаты в порядке добавления внутри сервера.
    pub position: u32,
}

/// Тело запроса для создания комнаты сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateServerRoomRequest {
    /// Человекочитаемое имя комнаты.
    pub name: String,
    /// Тип взаимодействия комнаты.
    pub kind: ServerRoomKind,
}

/// Тело запроса для обновления комнаты сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateServerRoomRequest {
    /// Человекочитаемое имя комнаты.
    pub name: String,
    /// Тип взаимодействия комнаты.
    pub kind: ServerRoomKind,
}

/// Ответ со списком комнат сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListServerRoomsResponse {
    /// Комнаты, доступные на сервере.
    pub rooms: Vec<ServerRoomSummary>,
}

/// Успешный ответ о создании комнаты сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateServerRoomResponse {
    /// Созданная комната.
    pub room: ServerRoomSummary,
}

/// Успешный ответ на обновление комнаты сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateServerRoomResponse {
    /// Обновленная комната.
    pub room: ServerRoomSummary,
}

/// Тело запроса для создания приглашения сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateServerInviteRequest {
    /// Необязательный максимальный лимит использований приглашения.
    pub max_uses: Option<u32>,
    /// Необязательный срок жизни приглашения в днях.
    pub expires_in_days: Option<u32>,
}

/// Успешный ответ о создании приглашения сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateServerInviteResponse {
    /// Стабильный код приглашения.
    pub code: String,
}

/// Данные приглашения сервера, возвращаемые lookup-эндпоинтами.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteSummary {
    /// Стабильный код приглашения.
    pub code: String,
    /// Количество успешных использований приглашения.
    pub uses: u32,
    /// Необязательный максимальный лимит использований приглашения.
    pub max_uses: Option<u32>,
    /// Необязательная временная метка истечения приглашения в формате RFC3339.
    pub expires_at: Option<String>,
}

/// Успешный ответ на поиск приглашения сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInviteInfoResponse {
    /// Метаданные приглашения.
    pub invite: ServerInviteSummary,
    /// Сервер, на который указывает приглашение.
    pub server: ServerSummary,
}

/// Успешный ответ на принятие приглашения сервера.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptServerInviteResponse {
    /// Сервер, к которому текущий пользователь теперь имеет доступ.
    pub server: ServerSummary,
    /// Был ли текущий пользователь уже активным участником.
    pub already_member: bool,
}

/// Ответ со списком серверов.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListServersResponse {
    /// Серверы, доступные текущему пользователю.
    pub servers: Vec<ServerSummary>,
}
