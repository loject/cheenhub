//! Text chat domain models.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Text message data used by text chat flows.
#[derive(Debug, Clone)]
pub(crate) struct TextMessage {
    /// Stable message identifier.
    pub(crate) id: Uuid,
    /// Server the message belongs to.
    pub(crate) server_id: Uuid,
    /// Room the message belongs to.
    pub(crate) room_id: Uuid,
    /// User that authored the message.
    pub(crate) author_user_id: Uuid,
    /// Author nickname snapshot.
    pub(crate) author_nickname: String,
    /// Message body.
    pub(crate) body: String,
    /// Image attachments included in the message.
    pub(crate) attachments: Vec<ChatAttachment>,
    /// Message creation timestamp.
    pub(crate) created_at: DateTime<Utc>,
    /// Soft-delete timestamp; set when a message is deleted.
    pub(crate) deleted_at: Option<DateTime<Utc>>,
    /// User who deleted the message; may differ from author for moderation deletes.
    pub(crate) deleted_by_user_id: Option<Uuid>,
}

/// Chat image attachment metadata.
#[derive(Debug, Clone)]
pub(crate) struct ChatAttachment {
    /// Stable attachment identifier.
    pub(crate) id: Uuid,
    /// Server the attachment belongs to.
    pub(crate) server_id: Uuid,
    /// Room the attachment belongs to.
    pub(crate) room_id: Uuid,
    /// User that uploaded the attachment.
    pub(crate) uploader_user_id: Uuid,
    /// Message that owns the attachment after it is sent.
    pub(crate) message_id: Option<Uuid>,
    /// S3 bucket that stores the object.
    pub(crate) bucket: String,
    /// S3 object key.
    pub(crate) object_key: String,
    /// Validated image content type.
    pub(crate) content_type: String,
    /// Original upload byte length.
    pub(crate) byte_size: i64,
    /// Image width in pixels.
    pub(crate) width: i32,
    /// Image height in pixels.
    pub(crate) height: i32,
    /// SHA-256 hash of uploaded bytes.
    pub(crate) sha256: String,
    /// Optional original filename from the upload request.
    pub(crate) original_filename: Option<String>,
    /// Creation timestamp.
    pub(crate) created_at: DateTime<Utc>,
}

/// New chat image attachment metadata.
#[derive(Debug, Clone)]
pub(crate) struct NewChatAttachment {
    /// Stable attachment identifier.
    pub(crate) id: Uuid,
    /// Server the attachment belongs to.
    pub(crate) server_id: Uuid,
    /// Room the attachment belongs to.
    pub(crate) room_id: Uuid,
    /// User that uploaded the attachment.
    pub(crate) uploader_user_id: Uuid,
    /// Message that owns the attachment after it is sent.
    pub(crate) message_id: Option<Uuid>,
    /// S3 bucket that stores the object.
    pub(crate) bucket: String,
    /// S3 object key.
    pub(crate) object_key: String,
    /// Validated image content type.
    pub(crate) content_type: String,
    /// Original upload byte length.
    pub(crate) byte_size: i64,
    /// Image width in pixels.
    pub(crate) width: i32,
    /// Image height in pixels.
    pub(crate) height: i32,
    /// SHA-256 hash of uploaded bytes.
    pub(crate) sha256: String,
    /// Optional original filename from the upload request.
    pub(crate) original_filename: Option<String>,
}
