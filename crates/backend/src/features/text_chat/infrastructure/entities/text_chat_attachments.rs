//! Сущность вложения текстового чата.

use sea_orm::entity::prelude::*;

/// Строка базы данных вложения текстового чата.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "text_chat_attachments")]
pub struct Model {
    /// Стабильный идентификатор вложения.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Сервер, которому принадлежит вложение.
    pub server_id: Uuid,
    /// Комната, которой принадлежит вложение.
    pub room_id: Uuid,
    /// Пользователь, загрузивший вложение.
    pub uploader_user_id: Uuid,
    /// Сообщение, которому принадлежит вложение после отправки.
    pub message_id: Option<Uuid>,
    /// S3 bucket, в котором хранится объект.
    pub bucket: String,
    /// Ключ S3-объекта.
    pub object_key: String,
    /// Проверенный MIME-тип изображения.
    pub content_type: String,
    /// Исходный размер загрузки в байтах.
    pub byte_size: i64,
    /// Ширина изображения в пикселях.
    pub width: i32,
    /// Высота изображения в пикселях.
    pub height: i32,
    /// SHA-256-хэш загруженных байтов.
    pub sha256: String,
    /// Необязательное исходное имя файла из запроса загрузки.
    pub original_filename: Option<String>,
    /// Временная метка создания.
    pub created_at: DateTimeUtc,
}

/// Связи вложения текстового чата.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
