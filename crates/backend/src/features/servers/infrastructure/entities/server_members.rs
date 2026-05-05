//! Server member entity.

use sea_orm::entity::prelude::*;

/// Server member database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_members")]
pub struct Model {
    /// Stable server member row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Server the member belongs to.
    pub server_id: Uuid,
    /// User that joined the server.
    pub user_id: Uuid,
    /// Membership start timestamp.
    pub joined_at: DateTimeUtc,
    /// Membership end timestamp for future soft leave.
    pub left_at: Option<DateTimeUtc>,
}

/// Server member relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
