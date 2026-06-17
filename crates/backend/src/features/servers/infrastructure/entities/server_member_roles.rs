//! Сущность назначения роли участнику сервера.

use sea_orm::entity::prelude::*;

/// Строка базы данных назначения роли участнику сервера.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "server_member_roles")]
pub struct Model {
    /// Сервер, которому принадлежит назначение.
    #[sea_orm(primary_key, auto_increment = false)]
    pub server_id: Uuid,
    /// Пользователь, который имеет роль.
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    /// Роль, назначенная пользователю.
    #[sea_orm(primary_key, auto_increment = false)]
    pub role_id: Uuid,
    /// Пользователь, который выдал роль.
    pub granted_by_user_id: Uuid,
    /// Когда роль была назначена.
    pub assigned_at: DateTimeUtc,
}

/// Связи назначения ролей участникам сервера.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
