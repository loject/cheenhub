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

/// Причина изменения social-состояния.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialChangeReason {
    /// Изменились друзья или заявки.
    Friends,
    /// Изменились личные сообщения или список диалогов.
    DirectMessages,
}
