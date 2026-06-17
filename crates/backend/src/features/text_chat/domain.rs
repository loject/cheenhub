//! Доменные модели текстового чата.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Данные текстового сообщения, используемые в потоках текстового чата.
#[derive(Debug, Clone)]
pub(crate) struct TextMessage {
    /// Стабильный идентификатор сообщения.
    pub(crate) id: Uuid,
    /// Сервер, которому принадлежит сообщение.
    pub(crate) server_id: Uuid,
    /// Комната, которой принадлежит сообщение.
    pub(crate) room_id: Uuid,
    /// Пользователь, создавший сообщение.
    pub(crate) author_user_id: Uuid,
    /// Снимок ника автора.
    pub(crate) author_nickname: String,
    /// Тело сообщения.
    pub(crate) body: String,
    /// Вложения-изображения, включенные в сообщение.
    pub(crate) attachments: Vec<ChatAttachment>,
    /// Временная метка создания сообщения.
    pub(crate) created_at: DateTime<Utc>,
    /// Временная метка мягкого удаления; задается при удалении сообщения.
    pub(crate) deleted_at: Option<DateTime<Utc>>,
    /// Пользователь, удаливший сообщение; для модераторских удалений может отличаться от автора.
    pub(crate) deleted_by_user_id: Option<Uuid>,
}

/// Метаданные вложения-изображения чата.
#[derive(Debug, Clone)]
pub(crate) struct ChatAttachment {
    /// Стабильный идентификатор вложения.
    pub(crate) id: Uuid,
    /// Сервер, которому принадлежит вложение.
    pub(crate) server_id: Uuid,
    /// Комната, которой принадлежит вложение.
    pub(crate) room_id: Uuid,
    /// Пользователь, загрузивший вложение.
    pub(crate) uploader_user_id: Uuid,
    /// Сообщение, которому принадлежит вложение после отправки.
    pub(crate) message_id: Option<Uuid>,
    /// S3 bucket, в котором хранится объект.
    pub(crate) bucket: String,
    /// Ключ S3-объекта.
    pub(crate) object_key: String,
    /// Проверенный MIME-тип изображения.
    pub(crate) content_type: String,
    /// Исходный размер загрузки в байтах.
    pub(crate) byte_size: i64,
    /// Ширина изображения в пикселях.
    pub(crate) width: i32,
    /// Высота изображения в пикселях.
    pub(crate) height: i32,
    /// SHA-256-хэш загруженных байтов.
    pub(crate) sha256: String,
    /// Необязательное исходное имя файла из запроса загрузки.
    pub(crate) original_filename: Option<String>,
    /// Временная метка создания.
    pub(crate) created_at: DateTime<Utc>,
}

/// Метаданные нового вложения-изображения чата.
#[derive(Debug, Clone)]
pub(crate) struct NewChatAttachment {
    /// Стабильный идентификатор вложения.
    pub(crate) id: Uuid,
    /// Сервер, которому принадлежит вложение.
    pub(crate) server_id: Uuid,
    /// Комната, которой принадлежит вложение.
    pub(crate) room_id: Uuid,
    /// Пользователь, загрузивший вложение.
    pub(crate) uploader_user_id: Uuid,
    /// Сообщение, которому принадлежит вложение после отправки.
    pub(crate) message_id: Option<Uuid>,
    /// S3 bucket, в котором хранится объект.
    pub(crate) bucket: String,
    /// Ключ S3-объекта.
    pub(crate) object_key: String,
    /// Проверенный MIME-тип изображения.
    pub(crate) content_type: String,
    /// Исходный размер загрузки в байтах.
    pub(crate) byte_size: i64,
    /// Ширина изображения в пикселях.
    pub(crate) width: i32,
    /// Высота изображения в пикселях.
    pub(crate) height: i32,
    /// SHA-256-хэш загруженных байтов.
    pub(crate) sha256: String,
    /// Необязательное исходное имя файла из запроса загрузки.
    pub(crate) original_filename: Option<String>,
}
