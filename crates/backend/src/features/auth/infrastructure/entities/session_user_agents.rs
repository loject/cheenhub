//! Auth session User-Agent entity.

use sea_orm::entity::prelude::*;

/// User-Agent observed for an auth session.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "session_user_agents")]
pub struct Model {
    /// Stable observed User-Agent row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Auth session that observed this User-Agent.
    pub session_id: Uuid,
    /// Normalized User-Agent string retained for future user-facing audit text.
    pub user_agent: String,
    /// First timestamp when this User-Agent was observed for the session.
    pub first_seen_at: DateTimeUtc,
    /// Last timestamp when this User-Agent was observed for the session.
    pub last_seen_at: DateTimeUtc,
}

/// Auth session User-Agent relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
