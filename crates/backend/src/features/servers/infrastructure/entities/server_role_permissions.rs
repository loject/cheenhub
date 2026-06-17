//! Сущность прав роли сервера.

use sea_orm::entity::prelude::*;

/// Строка базы данных права роли сервера.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_role_permissions")]
pub struct Model {
    /// Роль, которой принадлежит право.
    #[sea_orm(primary_key, auto_increment = false)]
    pub role_id: Uuid,
    /// Сохраненный ключ права.
    #[sea_orm(primary_key, auto_increment = false)]
    pub permission: String,
}

/// Связи прав роли сервера.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
