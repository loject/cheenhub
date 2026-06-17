//! Сущность использования приглашения сервера.

use sea_orm::entity::prelude::*;

/// Строка базы данных использования приглашения сервера.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_invite_uses")]
pub struct Model {
    /// Стабильный идентификатор строки использования приглашения.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Приглашение, которое было успешно использовано.
    pub invite_id: Uuid,
    /// Пользователь, успешно использовавший приглашение.
    pub user_id: Uuid,
    /// Временная метка использования приглашения.
    pub used_at: DateTimeUtc,
}

/// Связи использования приглашения сервера.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
