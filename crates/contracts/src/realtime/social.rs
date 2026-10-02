//! Realtime-контракты друзей и личных сообщений.
//!
//! Имена видов и payload'ов набора повторяют комнатные из `text_chat`, поэтому
//! типы переименованы в `DirectMessage*`: в одном модуле `realtime` два разных
//! набора нельзя экспортировать под одинаковым именем.

use serde::{Deserialize, Serialize};

use super::typing::TypingAuthor;

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
    /// Сообщить, что пользователь печатает сообщение в личном диалоге.
    StartDirectMessageTyping,
    /// Сообщить, что пользователь перестал печатать в личном диалоге.
    StopDirectMessageTyping,
    /// Запросить снимок участников диалога, которые сейчас печатают.
    DirectMessageTypingSnapshot,
    /// Событие о начале или завершении набора сообщения в личном диалоге.
    DirectMessageTypingChanged,
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

/// Полезная нагрузка запроса о начале набора сообщения в личном диалоге.
///
/// Отправляется один раз при переходе из состояния «не печатает» в состояние
/// «печатает» и повторяется как продление, пока пользователь печатает.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartDirectMessageTyping {
    /// Идентификатор личного диалога.
    pub conversation_id: String,
}

/// Полезная нагрузка запроса о завершении набора в личном диалоге.
///
/// Отправляется после отправки сообщения, очистки черновика или закрытия
/// вкладки, чтобы собеседник сразу перестал видеть индикатор набора.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopDirectMessageTyping {
    /// Идентификатор личного диалога.
    pub conversation_id: String,
}

/// Полезная нагрузка запроса снимка участников диалога, которые печатают.
///
/// Нужна при открытии диалога и после переподключения realtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageTypingSnapshotRequest {
    /// Идентификатор личного диалога.
    pub conversation_id: String,
}

/// Полезная нагрузка ответа со снимком участников диалога, которые печатают.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageTypingSnapshot {
    /// Идентификатор личного диалога.
    pub conversation_id: String,
    /// Участники, которые печатают сообщение прямо сейчас.
    pub typers: Vec<TypingAuthor>,
}

/// Полезная нагрузка события о начале или завершении набора в личном диалоге.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageTypingChanged {
    /// Идентификатор личного диалога.
    pub conversation_id: String,
    /// Участник, который начал или закончил печатать.
    pub author: TypingAuthor,
    /// Признак того, что участник печатает сообщение.
    pub is_typing: bool,
}
