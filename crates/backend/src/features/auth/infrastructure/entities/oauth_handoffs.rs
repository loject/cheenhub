//! OAuth frontend handoff entity.

use sea_orm::entity::prelude::*;

/// OAuth frontend handoff database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "oauth_handoffs")]
pub struct Model {
    /// Stable handoff row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// SHA-256 hash of the opaque handoff code.
    pub code_hash: String,
    /// Handoff result kind.
    pub kind: String,
    /// User id for authenticated or linked handoffs.
    pub user_id: Option<Uuid>,
    /// Registration intent id for registration handoffs.
    pub registration_intent_id: Option<Uuid>,
    /// Timestamp when the handoff was created.
    pub created_at: DateTimeUtc,
    /// Timestamp when the handoff expires.
    pub expires_at: DateTimeUtc,
    /// Timestamp when the handoff was consumed.
    pub consumed_at: Option<DateTimeUtc>,
}

/// OAuth frontend handoff relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
