//! Сущность приглашения сервера.

use sea_orm::entity::prelude::*;

/// Строка базы данных приглашения сервера.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_invites")]
pub struct Model {
    /// Стабильный идентификатор приглашения, используемый как код приглашения.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Сервер, которому принадлежит приглашение.
    pub server_id: Uuid,
    /// Пользователь, создавший приглашение.
    pub creator_user_id: Uuid,
    /// Необязательный максимальный лимит использований приглашения.
    pub max_uses: Option<i32>,
    /// Необязательная временная метка истечения приглашения.
    pub expires_at: Option<DateTimeUtc>,
    /// Временная метка создания приглашения.
    pub created_at: DateTimeUtc,
    /// Временная метка отзыва приглашения.
    pub revoked_at: Option<DateTimeUtc>,
}

/// Связи приглашения сервера.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
