//! Сущность роли сервера.

use sea_orm::entity::prelude::*;

/// Строка базы данных роли сервера.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "server_roles")]
pub struct Model {
    /// Стабильный идентификатор роли.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Сервер, которому принадлежит роль.
    pub server_id: Uuid,
    /// Человекочитаемое имя роли.
    pub name: String,
    /// Цвет роли в hex.
    pub color: String,
    /// Сохраненный вид роли.
    pub kind: String,
    /// Позиция в порядке внутри сервера.
    pub position: i32,
    /// Временная метка создания роли.
    pub created_at: DateTimeUtc,
    /// Временная метка последнего обновления роли.
    pub updated_at: DateTimeUtc,
}

/// Связи роли сервера.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
