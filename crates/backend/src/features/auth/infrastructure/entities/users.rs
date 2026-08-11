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
    pub password_hash: Option<String>,
    /// Current avatar image identifier.
    pub avatar_image_id: Option<Uuid>,
    /// Account registration timestamp.
    pub registered_at: DateTimeUtc,
    /// Last successful nickname update timestamp.
    pub nickname_updated_at: DateTimeUtc,
    /// Время исходного подтверждения правил из базовой схемы.
    /// Версии документов фиксируются отдельно в `legal_acceptances`.
    pub accepted_terms_at: DateTimeUtc,
    /// Last account update timestamp.
    pub updated_at: DateTimeUtc,
}

/// User account relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
