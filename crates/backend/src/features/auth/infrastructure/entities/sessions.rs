//! Auth session entity.

use sea_orm::entity::prelude::*;

/// Auth session database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    /// Stable session identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// User that owns the session.
    pub user_id: Uuid,
    /// Session creation timestamp.
    pub created_at: DateTimeUtc,
    /// Last activity timestamp.
    pub last_seen_at: DateTimeUtc,
    /// Session expiration timestamp.
    pub expires_at: DateTimeUtc,
    /// Session revocation timestamp.
    pub revoked_at: Option<DateTimeUtc>,
}

/// Auth session relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
