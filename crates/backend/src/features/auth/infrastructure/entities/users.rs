//! User account entity.

use sea_orm::entity::prelude::*;

/// User account database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    /// Stable user identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Public nickname shown to other users.
    pub nickname: String,
    /// Email address used for login.
    pub email: String,
    /// Normalized email used for lookup and uniqueness.
    pub email_normalized: String,
    /// Stored Argon2 password hash.
    pub password_hash: String,
    /// Account registration timestamp.
    pub registered_at: DateTimeUtc,
    // TODO: accepted_terms_at всегда совпадает с временем регистрации, может удалить?
    /// Mandatory policy acceptance timestamp.
    pub accepted_terms_at: DateTimeUtc,
    /// Last account update timestamp.
    pub updated_at: DateTimeUtc,
}

/// User account relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
