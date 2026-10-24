//! Команды ЛС и ответы Social поверх общего realtime-конверта.
//!
//! Все команды требуют `request_id`; пользователь определяется сессией.
//! Ответы используют существующие DTO ЛС, сохраняя одинаковый формат данных
//! для HTTP и realtime без дублирования моделей сообщений.

use serde::{Deserialize, Serialize};

/// Запрос списка личных диалогов проверенного пользователя.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListDirectMessageConversations;

/// Открывает диалог с другом; пользователя определяет realtime-сессия.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenDirectMessageConversation {
    /// Идентификатор друга, с которым открывается диалог.
    pub friend_user_id: String,
}

/// Запрос страницы истории; сообщения возвращаются от старых к новым.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadDirectMessageHistory {
    /// Идентификатор доступного пользователю диалога.
    pub conversation_id: String,
    /// Необязательный курсор для более старой страницы; без него загружается последняя.
    pub before_message_id: Option<String>,
}

/// Отправляет текст и необязательное ранее загруженное изображение.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendDirectMessage {
    /// Идентификатор личного диалога.
    pub conversation_id: String,
    /// Текст сообщения; пустой текст допустим только при наличии изображения.
    pub body: String,
    /// Идентификатор принадлежащего отправителю изображения этого диалога.
    pub image_id: Option<String>,
}

/// Изменяет текст собственного сообщения без изменения вложения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditDirectMessage {
    /// Идентификатор личного диалога.
    pub conversation_id: String,
    /// Идентификатор редактируемого сообщения.
    pub message_id: String,
    /// Новый текст; пустой допустим только для сообщения с изображением.
    pub body: String,
}

/// Продвигает checkpoint прочтения до сообщения; старый checkpoint не откатывает состояние.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkDirectMessageRead {
    /// Идентификатор личного диалога.
    pub conversation_id: String,
    /// Последнее прочитанное сообщение, принадлежащее этому диалогу.
    pub last_read_message_id: String,
}

/// Загружает изображение размером до 8 МиБ для последующего прикрепления.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadDirectMessageImage {
    /// Идентификатор личного диалога.
    pub conversation_id: String,
    /// Байты изображения в стандартном base64 с padding; лимит относится к декодированным байтам.
    pub data_base64: String,
}

/// Загружает изображение, прикреплённое к сообщению доступного диалога.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadDirectMessageImage {
    /// Идентификатор личного диалога.
    pub conversation_id: String,
    /// Идентификатор прикреплённого изображения.
    pub image_id: String,
}

/// Ответ с исходными байтами изображения; корреляция задаётся `request_id` конверта.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectMessageImageLoaded {
    /// Идентификатор изображения.
    pub id: String,
    /// MIME-тип изображения.
    pub content_type: String,
    /// Байты изображения в стандартном base64 с padding.
    pub data_base64: String,
}

/// Список личных диалогов текущего пользователя.
///
/// Формат данных совпадает с HTTP-ответом; `request_id` находится в realtime-конверте.
pub type DirectMessageConversations = crate::rest::ListDmConversationsResponse;

/// Открытый или существующий личный диалог.
///
/// Формат данных совпадает с HTTP-ответом; `request_id` находится в realtime-конверте.
pub type DirectMessageConversationOpened = crate::rest::OpenDmConversationResponse;

/// Страница истории с `has_more` и checkpoint прочтения собеседника.
///
/// Формат данных совпадает с HTTP-ответом; `request_id` находится в realtime-конверте.
pub type DirectMessageHistory = crate::rest::ListDmMessagesResponse;

/// Сохранённое сообщение с UUID, порядковым номером и состоянием доставки.
///
/// Формат данных совпадает с HTTP-ответом; `request_id` находится в realtime-конверте.
pub type SendDirectMessageAccepted = crate::rest::SendDmMessageResponse;

/// Сообщение с сохранённым текстом и временем редактирования.
///
/// Формат данных совпадает с HTTP-ответом; `request_id` находится в realtime-конверте.
pub type EditDirectMessageAccepted = crate::rest::EditDmMessageResponse;

/// Актуальный checkpoint прочтения и счётчики непрочитанных сообщений.
///
/// Формат данных совпадает с HTTP-ответом; `request_id` находится в realtime-конверте.
pub type MarkDirectMessageReadAccepted = crate::rest::MarkDmConversationReadResponse;

/// Метаданные изображения, которое можно прикрепить к сообщению.
///
/// Формат данных совпадает с HTTP-ответом; `request_id` находится в realtime-конверте.
pub type UploadDirectMessageImageAccepted = crate::rest::UploadDmImageResponse;
