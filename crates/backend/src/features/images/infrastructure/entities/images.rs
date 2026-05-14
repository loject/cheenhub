//! Stored image entity.

use sea_orm::entity::prelude::*;

/// Stored image database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "images")]
pub struct Model {
    /// Stable image identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// User that owns this image.
    pub owner_user_id: Uuid,
    /// Image purpose, such as `user_avatar`.
    pub kind: String,
    /// Stored image MIME content type.
    pub content_type: String,
    /// Pixel width.
    pub width: i32,
    /// Pixel height.
    pub height: i32,
    /// Stored byte size.
    pub byte_size: i64,
    /// Hex-encoded SHA-256 digest of stored bytes.
    pub sha256: String,
    /// Storage backend name, such as `database`.
    pub storage_backend: String,
    /// External object storage key when bytes are not stored in this row.
    pub storage_key: Option<String>,
    /// Stored image bytes for database-backed images.
    pub data: Option<Vec<u8>>,
    /// Creation timestamp.
    pub created_at: DateTimeUtc,
    /// Last update timestamp.
    pub updated_at: DateTimeUtc,
}

/// Stored image relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
