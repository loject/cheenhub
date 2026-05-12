//! Linked OAuth account entity.

use sea_orm::entity::prelude::*;

/// Linked OAuth account database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "oauth_accounts")]
pub struct Model {
    /// Stable linked account row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// CheenHub user that owns the linked account.
    pub user_id: Uuid,
    /// External OAuth provider name.
    pub provider: String,
    /// Stable provider-side subject identifier.
    pub provider_subject: String,
    /// Email address reported by the provider.
    pub email: String,
    /// Display name reported by the provider.
    pub display_name: Option<String>,
    /// Timestamp when the provider was linked.
    pub linked_at: DateTimeUtc,
}

/// Linked OAuth account relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
