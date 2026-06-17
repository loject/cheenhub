//! Сущность комнаты сервера.

use sea_orm::entity::prelude::*;

/// Строка базы данных комнаты сервера.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_rooms")]
pub struct Model {
    /// Стабильный идентификатор комнаты.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Сервер, которому принадлежит комната.
    pub server_id: Uuid,
    /// Человекочитаемое имя комнаты.
    pub name: String,
    /// Сохраненный вид комнаты.
    pub kind: String,
    /// Позиция в порядке добавления внутри сервера.
    pub position: i32,
    /// Временная метка создания комнаты.
    pub created_at: DateTimeUtc,
    /// Временная метка последнего обновления комнаты.
    pub updated_at: DateTimeUtc,
}

/// Связи комнаты сервера.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
