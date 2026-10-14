//! SeaORM-сущность закрепления сообщения.

use sea_orm::entity::prelude::*;

/// Запись закрепления сообщения в комнате.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "text_message_pins")]
pub struct Model {
    /// Идентификатор закреплённого сообщения.
    #[sea_orm(primary_key, auto_increment = false)]
    pub message_id: Uuid,
    /// Сервер, которому принадлежит сообщение.
    pub server_id: Uuid,
    /// Комната, которой принадлежит сообщение.
    pub room_id: Uuid,
    /// Пользователь, закрепивший сообщение.
    pub pinned_by_user_id: Uuid,
    /// Время закрепления.
    pub pinned_at: DateTimeUtc,
}

/// Сущность не имеет используемых ORM-связей.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
