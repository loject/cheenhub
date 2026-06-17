//! Сущность сервера.

use sea_orm::entity::prelude::*;

/// Строка базы данных сервера.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "servers")]
pub struct Model {
    /// Стабильный идентификатор сервера.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Пользователь, которому принадлежит сервер.
    pub owner_user_id: Uuid,
    /// Человекочитаемое имя сервера.
    pub name: String,
    /// Идентификатор сохраненного изображения аватара сервера.
    pub avatar_image_id: Option<Uuid>,
    /// Временная метка создания сервера.
    pub created_at: DateTimeUtc,
    /// Временная метка последнего обновления сервера.
    pub updated_at: DateTimeUtc,
}

/// Связи сервера.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
