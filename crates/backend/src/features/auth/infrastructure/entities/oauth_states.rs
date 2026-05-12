//! OAuth authorization state entity.

use sea_orm::entity::prelude::*;

/// OAuth authorization state database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "oauth_states")]
pub struct Model {
    /// Stable OAuth state row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// SHA-256 hash of the opaque state value.
    pub state_hash: String,
    /// OAuth nonce sent to the provider.
    pub nonce: String,
    /// Flow kind, such as login or link.
    pub flow_kind: String,
    /// Authenticated user for link flows.
    pub user_id: Option<Uuid>,
    /// Timestamp when the state was created.
    pub created_at: DateTimeUtc,
    /// Timestamp when the state expires.
    pub expires_at: DateTimeUtc,
    /// Timestamp when the state was consumed.
    pub consumed_at: Option<DateTimeUtc>,
}

/// OAuth authorization state relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
