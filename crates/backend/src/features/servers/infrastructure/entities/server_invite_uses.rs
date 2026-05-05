//! Server invite use entity.

use sea_orm::entity::prelude::*;

/// Server invite use database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_invite_uses")]
pub struct Model {
    /// Stable invite use row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Invite that was used successfully.
    pub invite_id: Uuid,
    /// User that used the invite successfully.
    pub user_id: Uuid,
    /// Invite use timestamp.
    pub used_at: DateTimeUtc,
}

/// Server invite use relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
