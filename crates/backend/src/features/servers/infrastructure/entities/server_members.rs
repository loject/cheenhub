//! Сущность участника сервера.

use sea_orm::entity::prelude::*;

/// Строка базы данных участника сервера.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_members")]
pub struct Model {
    /// Стабильный идентификатор строки участника сервера.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Сервер, которому принадлежит участник.
    pub server_id: Uuid,
    /// Пользователь, вступивший на сервер.
    pub user_id: Uuid,
    /// Временная метка начала участия.
    pub joined_at: DateTimeUtc,
    /// Временная метка окончания участия для будущего мягкого выхода.
    pub left_at: Option<DateTimeUtc>,
}

/// Связи участника сервера.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
