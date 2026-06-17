//! Контракты realtime-модуля текстового чата.

use serde::{Deserialize, Serialize};

/// Виды сообщений модуля текстового чата.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextChatKind {
    /// Загрузить свежую историю сообщений комнаты.
    LoadRoomHistory,
    /// Ответ с историей сообщений комнаты.
    RoomHistory,
    /// Отправить сообщение в комнату.
    SendMessage,
    /// Подтверждает, что сообщение принято для рассылки и сохранения.
    SendMessageAccepted,
    /// Загрузить вложение-изображение чата.
    UploadImage,
    /// Подтверждает, что изображение чата загружено.
    UploadImageAccepted,
    /// Загрузить вложение-изображение чата через realtime.
    LoadImage,
    /// Ответ с одним вложением-изображением чата.
    ImageLoaded,
    /// Событие о новом сообщении.
    MessageCreated,
    /// Удалить одно из собственных сообщений пользователя.
    DeleteMessage,
    /// Подтверждает, что удаление сообщения принято.
    DeleteMessageAccepted,
    /// Сообщение удалено автором; получатели должны убрать его.
    MessageDeleted,
}

/// Полезная нагрузка запроса для загрузки истории комнаты.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadRoomHistory {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Идентификатор сообщения, перед которым нужно загрузить сообщения.
    pub before_message_id: Option<String>,
}

/// Полезная нагрузка ответа с последними сообщениями комнаты.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomHistory {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Последние сохраненные сообщения комнаты.
    pub messages: Vec<TextChatMessage>,
    /// Доступны ли более старые сообщения до этой страницы.
    pub has_more: bool,
}

/// Полезная нагрузка запроса для отправки сообщения в комнату.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendMessage {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Тело сообщения.
    pub body: String,
    /// Идентификаторы загруженных вложений-изображений, которые нужно включить в сообщение.
    #[serde(default)]
    pub attachment_ids: Vec<String>,
}

/// Полезная нагрузка ответа после принятия отправки сообщения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendMessageAccepted {
    /// Принятое сообщение.
    pub message: TextChatMessage,
}

/// Полезная нагрузка запроса для загрузки одного вложения-изображения чата.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadChatImage {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Необязательное исходное имя файла.
    pub original_filename: Option<String>,
    /// Байты изображения в Base64.
    pub data_base64: String,
}

/// Ответ после загрузки изображения чата.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatImageUploadResponse {
    /// Стабильный идентификатор вложения.
    pub id: String,
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Проверенный MIME-тип изображения.
    pub content_type: String,
    /// Загруженный размер в байтах.
    pub byte_size: i64,
    /// Ширина изображения в пикселях.
    pub width: i32,
    /// Высота изображения в пикселях.
    pub height: i32,
}

/// Метаданные вложения-изображения, включаемые в сообщение текстового чата.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextChatImageAttachment {
    /// Стабильный идентификатор вложения.
    pub id: String,
    /// Проверенный MIME-тип изображения.
    pub content_type: String,
    /// Загруженный размер в байтах.
    pub byte_size: i64,
    /// Ширина изображения в пикселях.
    pub width: i32,
    /// Высота изображения в пикселях.
    pub height: i32,
}

/// Полезная нагрузка запроса для загрузки одного вложения-изображения чата.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadChatImage {
    /// Стабильный идентификатор вложения.
    pub attachment_id: String,
}

/// Полезная нагрузка ответа с одним вложением-изображением чата.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatImageLoadedResponse {
    /// Стабильный идентификатор вложения.
    pub id: String,
    /// Проверенный MIME-тип изображения.
    pub content_type: String,
    /// Байты изображения в Base64.
    pub data_base64: String,
}

/// Полезная нагрузка запроса для мягкого удаления одного из собственных сообщений пользователя.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteMessage {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Идентификатор сообщения для удаления.
    pub message_id: String,
}

/// Полезная нагрузка ответа после принятия удаления сообщения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteMessageAccepted {
    /// Идентификатор удаленного сообщения.
    pub message_id: String,
}

/// Полезная нагрузка широковещания, уведомляющая участников комнаты об удалении сообщения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDeletedPayload {
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Идентификатор удаленного сообщения.
    pub message_id: String,
}

/// Полезная нагрузка сообщения текстового чата.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextChatMessage {
    /// Стабильный идентификатор сообщения.
    pub id: String,
    /// Идентификатор сервера.
    pub server_id: String,
    /// Идентификатор комнаты.
    pub room_id: String,
    /// Идентификатор пользователя-автора.
    pub author_user_id: String,
    /// Снимок ника автора.
    pub author_nickname: String,
    /// Публичный URL аватара автора, если он настроен.
    pub author_avatar_url: Option<String>,
    /// Тело сообщения.
    pub body: String,
    /// Вложения-изображения, включенные в сообщение.
    #[serde(default)]
    pub attachments: Vec<TextChatImageAttachment>,
    /// Временная метка создания сообщения в формате RFC3339.
    pub created_at: String,
}
