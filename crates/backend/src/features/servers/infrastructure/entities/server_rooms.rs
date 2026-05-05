//! Server room entity.

use sea_orm::entity::prelude::*;

/// Server room database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_rooms")]
pub struct Model {
    /// Stable room identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Server the room belongs to.
    pub server_id: Uuid,
    /// Human-readable room name.
    pub name: String,
    /// Stored room kind.
    pub kind: String,
    /// Append-only ordering position inside the server.
    pub position: i32,
    /// Room creation timestamp.
    pub created_at: DateTimeUtc,
    /// Last room update timestamp.
    pub updated_at: DateTimeUtc,
}

/// Server room relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
