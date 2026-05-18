//! Server member exclusion entity.

use sea_orm::entity::prelude::*;

/// Server member exclusion database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_member_exclusions")]
pub struct Model {
    /// Stable exclusion row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Server the exclusion belongs to.
    pub server_id: Uuid,
    /// User blocked from rejoining.
    pub user_id: Uuid,
    /// User or system actor that created the exclusion.
    pub initiator_user_id: Uuid,
    /// Timestamp until which the user cannot rejoin.
    pub expires_at: DateTimeUtc,
    /// Exclusion creation timestamp.
    pub created_at: DateTimeUtc,
}

/// Server member exclusion relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
