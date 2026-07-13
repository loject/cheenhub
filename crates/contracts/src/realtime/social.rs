//! Realtime-контракты друзей и личных сообщений.

use serde::{Deserialize, Serialize};

/// Тип realtime-сообщения social-модуля.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialKind {
    /// Подписка вкладки на social-события текущего пользователя.
    Subscribe,
    /// Подписка на social-события активна.
    Ready,
    /// У текущего пользователя изменились друзья, заявки или личные сообщения.
    Changed,
    /// Получатель получил новое личное сообщение.
    DirectMessageCreated,
    /// Участник подтвердил прочтение личного диалога.
    ConversationReadCheckpoint,
}

/// Пустой запрос подписки на social-события.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscribeSocial;

/// Ответ на успешную подписку social-модуля.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialReady;

/// Realtime-событие изменения social-состояния.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialChanged {
    /// Причина обновления, полезная для диагностики клиента.
    pub reason: SocialChangeReason,
    /// Идентификатор личного диалога, если изменение относится к ЛС.
    pub conversation_id: Option<String>,
}

/// Realtime-событие нового личного сообщения для получателя.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageCreated {
    /// Стабильный идентификатор сообщения.
    pub message_id: String,
    /// Идентификатор личного диалога.
    pub conversation_id: String,
    /// Монотонный порядковый номер сообщения внутри диалога.
    pub message_seq: i64,
    /// Идентификатор отправителя.
    pub sender_user_id: String,
    /// Никнейм отправителя на момент доставки события.
    pub sender_nickname: String,
    /// Текст сообщения.
    pub body: String,
    /// Серверное время создания сообщения в формате RFC3339.
    pub created_at: String,
}

/// Realtime-событие подтверждения прочтения личного диалога.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationReadCheckpoint {
    /// Идентификатор личного диалога.
    pub conversation_id: String,
    /// Пользователь, который прочитал сообщения.
    pub reader_user_id: String,
    /// Последнее прочитанное сообщение.
    pub last_read_message_id: String,
    /// Последний прочитанный порядковый номер.
    pub last_read_seq: i64,
    /// Серверное время подтверждения прочтения в формате RFC3339.
    pub read_at: String,
}

/// Причина изменения social-состояния.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialChangeReason {
    /// Изменились друзья или заявки.
    Friends,
    /// Изменились личные сообщения или список диалогов.
    DirectMessages,
}
