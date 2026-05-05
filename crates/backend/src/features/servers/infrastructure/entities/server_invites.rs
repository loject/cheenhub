//! Server invite entity.

use sea_orm::entity::prelude::*;

/// Server invite database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_invites")]
pub struct Model {
    /// Stable invite identifier used as the invite code.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Server the invite belongs to.
    pub server_id: Uuid,
    /// User that created the invite.
    pub creator_user_id: Uuid,
    /// Optional maximum number of accepted invite uses.
    pub max_uses: Option<i32>,
    /// Optional invite expiration timestamp.
    pub expires_at: Option<DateTimeUtc>,
    /// Invite creation timestamp.
    pub created_at: DateTimeUtc,
}

/// Server invite relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
