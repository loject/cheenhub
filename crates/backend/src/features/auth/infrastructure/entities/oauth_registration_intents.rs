//! OAuth registration intent entity.

use sea_orm::entity::prelude::*;

/// OAuth registration intent database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "oauth_registration_intents")]
pub struct Model {
    /// Stable registration intent row identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// External OAuth provider name.
    pub provider: String,
    /// Stable provider-side subject identifier.
    pub provider_subject: String,
    /// Verified email address reported by the provider.
    pub email: String,
    /// Display name reported by the provider.
    pub display_name: Option<String>,
    /// Timestamp when the intent was created.
    pub created_at: DateTimeUtc,
    /// Timestamp when the intent expires.
    pub expires_at: DateTimeUtc,
    /// Timestamp when the intent was consumed.
    pub consumed_at: Option<DateTimeUtc>,
}

/// OAuth registration intent relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
