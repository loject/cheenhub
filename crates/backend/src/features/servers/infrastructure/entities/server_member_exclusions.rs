//! Сущность исключения участника сервера.

use sea_orm::entity::prelude::*;

/// Строка базы данных исключения участника сервера.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_member_exclusions")]
pub struct Model {
    /// Стабильный идентификатор строки исключения.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Сервер, которому принадлежит исключение.
    pub server_id: Uuid,
    /// Пользователь, которому запрещен повторный вход.
    pub user_id: Uuid,
    /// Пользователь или системный актер, создавший исключение.
    pub initiator_user_id: Uuid,
    /// Временная метка, до которой пользователь не может вернуться.
    pub expires_at: DateTimeUtc,
    /// Временная метка создания исключения.
    pub created_at: DateTimeUtc,
}

/// Связи исключения участника сервера.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
