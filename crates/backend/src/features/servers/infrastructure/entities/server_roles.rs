//! Server role entity.

use sea_orm::entity::prelude::*;

/// Server role database row.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "server_roles")]
pub struct Model {
    /// Stable role identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Server the role belongs to.
    pub server_id: Uuid,
    /// Human-readable role name.
    pub name: String,
    /// Hex role color.
    pub color: String,
    /// Stored role kind.
    pub kind: String,
    /// Ordering position inside the server.
    pub position: i32,
    /// Role creation timestamp.
    pub created_at: DateTimeUtc,
    /// Last role update timestamp.
    pub updated_at: DateTimeUtc,
}

/// Server role relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
