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
    /// Message creation timestamp.
    pub(crate) created_at: DateTime<Utc>,
    /// Soft-delete timestamp; set when a message is deleted.
    pub(crate) deleted_at: Option<DateTime<Utc>>,
    /// User who deleted the message; may differ from author for moderation deletes.
    pub(crate) deleted_by_user_id: Option<Uuid>,
}
