//! Server member role assignment entity.

use sea_orm::entity::prelude::*;

/// Server member role assignment database row.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "server_member_roles")]
pub struct Model {
    /// Server the assignment belongs to.
    #[sea_orm(primary_key, auto_increment = false)]
    pub server_id: Uuid,
    /// User that holds the role.
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Uuid,
    /// Role assigned to the user.
    #[sea_orm(primary_key, auto_increment = false)]
    pub role_id: Uuid,
    /// User that granted the role.
    pub granted_by_user_id: Uuid,
    /// When the role was assigned.
    pub assigned_at: DateTimeUtc,
}

/// Server member role relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
