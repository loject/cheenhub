//! Server entity.

use sea_orm::entity::prelude::*;

/// Server database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "servers")]
pub struct Model {
    /// Stable server identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// User that owns the server.
    pub owner_user_id: Uuid,
    /// Human-readable server name.
    pub name: String,
    /// Server creation timestamp.
    pub created_at: DateTimeUtc,
    /// Last server update timestamp.
    pub updated_at: DateTimeUtc,
}

/// Server relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
