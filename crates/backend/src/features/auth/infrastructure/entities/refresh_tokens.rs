//! Refresh token entity.

use sea_orm::entity::prelude::*;

/// Refresh token database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "refresh_tokens")]
pub struct Model {
    /// Stable refresh token row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Session that owns the refresh token.
    pub session_id: Uuid,
    /// SHA-256 hash of the opaque refresh token.
    pub token_hash: String,
    /// Refresh token creation timestamp.
    pub created_at: DateTimeUtc,
    /// Refresh token rotation timestamp.
    pub rotated_at: Option<DateTimeUtc>,
    /// Refresh token expiration timestamp.
    pub expires_at: DateTimeUtc,
    /// Refresh token revocation timestamp.
    pub revoked_at: Option<DateTimeUtc>,
}

/// Refresh token relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
