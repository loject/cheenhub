//! Password reset token entity.

use sea_orm::entity::prelude::*;

/// Password reset token database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "password_reset_tokens")]
pub struct Model {
    /// Stable reset token row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// User that owns the reset token.
    pub user_id: Uuid,
    /// SHA-256 hash of the opaque reset token.
    pub token_hash: String,
    /// Timestamp when the reset token was created.
    pub created_at: DateTimeUtc,
    /// Timestamp when the reset token expires.
    pub expires_at: DateTimeUtc,
    /// Timestamp when the reset token was consumed.
    pub consumed_at: Option<DateTimeUtc>,
}

/// Password reset token relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
