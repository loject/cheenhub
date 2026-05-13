//! User nickname change history entity.

use sea_orm::entity::prelude::*;

/// User nickname change history database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "user_nickname_history")]
pub struct Model {
    /// Stable history row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// User whose nickname changed.
    pub user_id: Uuid,
    /// Auth session used to perform the change.
    pub session_id: Uuid,
    /// Nickname before the change.
    pub old_nickname: String,
    /// Nickname after the change.
    pub new_nickname: String,
    /// Timestamp when the nickname changed.
    pub changed_at: DateTimeUtc,
}

/// User nickname history relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
