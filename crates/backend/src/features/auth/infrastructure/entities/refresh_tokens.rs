//! Сущность refresh-токена.

use sea_orm::entity::prelude::*;

/// Строка базы данных refresh-токена.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "refresh_tokens")]
pub struct Model {
    /// Стабильный идентификатор строки refresh-токена.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Сессия, которой принадлежит refresh-токен.
    pub session_id: Uuid,
    /// SHA-256-хэш непрозрачного refresh-токена.
    pub token_hash: String,
    /// Временная метка создания refresh-токена.
    pub created_at: DateTimeUtc,
    /// Временная метка ротации refresh-токена.
    pub rotated_at: Option<DateTimeUtc>,
    /// Временная метка истечения refresh-токена.
    pub expires_at: DateTimeUtc,
    /// Временная метка отзыва refresh-токена.
    pub revoked_at: Option<DateTimeUtc>,
}

/// Связи refresh-токена.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
