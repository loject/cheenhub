//! User password change trace entity.

use sea_orm::entity::prelude::*;

/// User password change trace database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "user_password_change_trace")]
pub struct Model {
    /// Stable trace row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// User whose password changed.
    pub user_id: Uuid,
    /// Auth session used to perform the change.
    pub session_id: Uuid,
    /// Timestamp when the password changed.
    pub changed_at: DateTimeUtc,
}

/// User password change trace relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
