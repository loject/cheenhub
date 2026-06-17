//! Сущность текстового сообщения.

use sea_orm::entity::prelude::*;

/// Строка базы данных текстового сообщения.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "text_messages")]
pub struct Model {
    /// Стабильный идентификатор сообщения.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Сервер, которому принадлежит сообщение.
    pub server_id: Uuid,
    /// Комната, которой принадлежит сообщение.
    pub room_id: Uuid,
    /// Пользователь, создавший сообщение.
    pub author_user_id: Uuid,
    /// Снимок ника автора.
    pub author_nickname: String,
    /// Тело сообщения.
    pub body: String,
    /// Временная метка создания сообщения.
    pub created_at: DateTimeUtc,
    /// Временная метка мягкого удаления; задается при удалении сообщения.
    pub deleted_at: Option<DateTimeUtc>,
    /// Пользователь, удаливший сообщение; для модераторских удалений может отличаться от автора.
    pub deleted_by_user_id: Option<Uuid>,
}

/// Связи текстового сообщения.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
