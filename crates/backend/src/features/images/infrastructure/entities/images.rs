//! Сущность сохраненного изображения.

use sea_orm::entity::prelude::*;

/// Строка базы данных сохраненного изображения.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "images")]
pub struct Model {
    /// Стабильный идентификатор изображения.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Пользователь, которому принадлежит это изображение.
    pub owner_user_id: Uuid,
    /// Назначение изображения, например `user_avatar`.
    pub kind: String,
    /// Сохраненный MIME-тип изображения.
    pub content_type: String,
    /// Ширина в пикселях.
    pub width: i32,
    /// Высота в пикселях.
    pub height: i32,
    /// Сохраненный размер в байтах.
    pub byte_size: i64,
    /// SHA-256-digest сохраненных байтов в hex-формате.
    pub sha256: String,
    /// Имя бэкенда хранения, например `database`.
    pub storage_backend: String,
    /// Ключ внешнего объектного хранилища, когда байты не хранятся в этой строке.
    pub storage_key: Option<String>,
    /// Сохраненные байты изображения для изображений, хранящихся в базе данных.
    pub data: Option<Vec<u8>>,
    /// Временная метка создания.
    pub created_at: DateTimeUtc,
    /// Временная метка последнего обновления.
    pub updated_at: DateTimeUtc,
}

/// Связи сохраненного изображения.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
