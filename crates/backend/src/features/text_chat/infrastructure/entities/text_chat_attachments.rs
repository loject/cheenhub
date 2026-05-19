//! Text chat attachment entity.

use sea_orm::entity::prelude::*;

/// Text chat attachment database row.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "text_chat_attachments")]
pub struct Model {
    /// Stable attachment identifier.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Server the attachment belongs to.
    pub server_id: Uuid,
    /// Room the attachment belongs to.
    pub room_id: Uuid,
    /// User that uploaded the attachment.
    pub uploader_user_id: Uuid,
    /// Message that owns the attachment after it is sent.
    pub message_id: Option<Uuid>,
    /// S3 bucket that stores the object.
    pub bucket: String,
    /// S3 object key.
    pub object_key: String,
    /// Validated image content type.
    pub content_type: String,
    /// Original upload byte length.
    pub byte_size: i64,
    /// Image width in pixels.
    pub width: i32,
    /// Image height in pixels.
    pub height: i32,
    /// SHA-256 hash of uploaded bytes.
    pub sha256: String,
    /// Optional original filename from the upload request.
    pub original_filename: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTimeUtc,
}

/// Text chat attachment relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
