//! Контракты REST для друзей и личных сообщений.

use serde::{Deserialize, Serialize};

/// Отношение найденного пользователя к текущему пользователю.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRelationStatus {
    /// Пользователь уже находится в друзьях.
    Friends,
    /// Текущий пользователь отправил заявку.
    PendingOutgoing,
    /// Текущий пользователь получил заявку.
    PendingIncoming,
}

/// Статус заявки или связи дружбы.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FriendRequestStatus {
    /// Заявка ожидает решения получателя.
    Pending,
    /// Заявка принята, пользователи стали друзьями.
    Accepted,
    /// Получатель отклонил заявку.
    Declined,
    /// Отправитель отменил заявку или дружба была удалена.
    Cancelled,
}

/// Пользователь, найденный поиском друзей.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSearchResult {
    /// Стабильный идентификатор пользователя.
    pub id: String,
    /// Публичный никнейм пользователя.
    pub nickname: String,
    /// Публичный URL аватара, если он настроен.
    pub avatar_url: Option<String>,
    /// Текущее отношение к пользователю, если оно уже есть.
    pub relation: Option<UserRelationStatus>,
}

/// Ответ со списком найденных пользователей.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchUsersResponse {
    /// Найденные пользователи.
    pub users: Vec<UserSearchResult>,
}

/// Краткие данные друга.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendSummary {
    /// Идентификатор пользователя-друга.
    pub user_id: String,
    /// Публичный никнейм друга.
    pub nickname: String,
    /// Публичный URL аватара друга, если он настроен.
    pub avatar_url: Option<String>,
    /// Количество непрочитанных личных сообщений от этого друга.
    pub unread_count: i64,
    /// Временная метка начала дружбы в формате RFC3339.
    pub friends_since: String,
}

/// Краткие данные заявки в друзья.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendRequestSummary {
    /// Стабильный идентификатор заявки.
    pub id: String,
    /// Пользователь, отправивший заявку.
    pub sender_user_id: String,
    /// Никнейм отправителя.
    pub sender_nickname: String,
    /// URL аватара отправителя, если он настроен.
    pub sender_avatar_url: Option<String>,
    /// Пользователь, получивший заявку.
    pub recipient_user_id: String,
    /// Никнейм получателя.
    pub recipient_nickname: String,
    /// URL аватара получателя, если он настроен.
    pub recipient_avatar_url: Option<String>,
    /// Текущий статус заявки.
    pub status: FriendRequestStatus,
    /// Временная метка создания в формате RFC3339.
    pub created_at: String,
    /// Временная метка последнего обновления в формате RFC3339.
    pub updated_at: String,
}

/// Ответ со списком друзей.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListFriendsResponse {
    /// Друзья текущего пользователя.
    pub friends: Vec<FriendSummary>,
}

/// Ответ со списком заявок в друзья.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListFriendRequestsResponse {
    /// Заявки в друзья.
    pub requests: Vec<FriendRequestSummary>,
}

/// Запрос на отправку заявки в друзья.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendFriendRequestRequest {
    /// Пользователь, которому отправляется заявка.
    pub recipient_user_id: String,
}

/// Ответ на отправку или изменение заявки в друзья.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendFriendRequestResponse {
    /// Созданная или обновленная заявка.
    pub request: FriendRequestSummary,
}

/// Краткие данные диалога личных сообщений.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmConversationSummary {
    /// Стабильный идентификатор диалога.
    pub id: String,
    /// Идентификатор второго участника.
    pub friend_user_id: String,
    /// Никнейм второго участника.
    pub friend_nickname: String,
    /// URL аватара второго участника, если он настроен.
    pub friend_avatar_url: Option<String>,
    /// Количество непрочитанных сообщений без UI-ограничения.
    pub unread_count: i64,
    /// Последнее прочитанное сообщение текущего пользователя.
    pub last_read_message_id: Option<String>,
    /// Последний прочитанный порядковый номер текущего пользователя.
    pub last_read_seq: i64,
    /// Время последнего подтверждения прочтения в формате RFC3339.
    pub last_read_at: Option<String>,
    /// Временная метка последнего обновления в формате RFC3339.
    pub updated_at: String,
}

/// Статус доставки исходящего личного сообщения.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DmMessageDeliveryStatus {
    /// Сервер принял и сохранил сообщение.
    Accepted,
    /// Получатель прочитал сообщение.
    Read,
}

/// Краткие данные сообщения личного диалога.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmMessageSummary {
    /// Стабильный идентификатор сообщения.
    pub id: String,
    /// Идентификатор диалога.
    pub conversation_id: String,
    /// Монотонный порядковый номер сообщения внутри диалога.
    pub seq: i64,
    /// Идентификатор отправителя.
    pub sender_user_id: String,
    /// Никнейм отправителя.
    pub sender_nickname: String,
    /// URL аватара отправителя, если он настроен.
    pub sender_avatar_url: Option<String>,
    /// Текст сообщения.
    pub body: String,
    /// Статус доставки для текущего пользователя, если сообщение исходящее.
    pub delivery_status: Option<DmMessageDeliveryStatus>,
    /// Временная метка создания в формате RFC3339.
    pub created_at: String,
}

/// Ответ со списком диалогов личных сообщений.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListDmConversationsResponse {
    /// Диалоги текущего пользователя.
    pub conversations: Vec<DmConversationSummary>,
}

/// Запрос на открытие диалога личных сообщений.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenDmConversationRequest {
    /// Друг, с которым нужно открыть диалог.
    pub friend_user_id: String,
}

/// Ответ на открытие диалога личных сообщений.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenDmConversationResponse {
    /// Открытый или существующий диалог.
    pub conversation: DmConversationSummary,
}

/// Ответ со страницей сообщений личного диалога.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListDmMessagesResponse {
    /// Сообщения в порядке от старых к новым.
    pub messages: Vec<DmMessageSummary>,
    /// Последний прочитанный порядковый номер собеседника.
    pub recipient_last_read_seq: i64,
    /// Есть ли более старые сообщения перед этой страницей.
    pub has_more: bool,
}

/// Запрос на отметку личного диалога прочитанным.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkDmConversationReadRequest {
    /// Последнее сообщение, до которого пользователь дочитал.
    pub last_read_message_id: String,
}

/// Ответ на отметку личного диалога прочитанным.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkDmConversationReadResponse {
    /// Идентификатор диалога.
    pub conversation_id: String,
    /// Последнее прочитанное сообщение.
    pub last_read_message_id: Option<String>,
    /// Последний прочитанный порядковый номер.
    pub last_read_seq: i64,
    /// Серверное время последнего подтверждения прочтения.
    pub last_read_at: Option<String>,
    /// Количество непрочитанных сообщений в этом диалоге.
    pub conversation_unread_count: i64,
    /// Суммарное количество непрочитанных личных сообщений.
    pub total_unread_count: i64,
}

/// Запрос на отправку личного сообщения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendDmMessageRequest {
    /// Текст сообщения.
    pub body: String,
}

/// Ответ на отправку личного сообщения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendDmMessageResponse {
    /// Созданное сообщение.
    pub message: DmMessageSummary,
}
