//! Server role permission entity.

use sea_orm::entity::prelude::*;

/// Server role permission database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "server_role_permissions")]
pub struct Model {
    /// Role that owns the permission.
    #[sea_orm(primary_key, auto_increment = false)]
    pub role_id: Uuid,
    /// Stored permission key.
    #[sea_orm(primary_key, auto_increment = false)]
    pub permission: String,
}

/// Server role permission relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
