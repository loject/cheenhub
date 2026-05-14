//! Image domain models.

use uuid::Uuid;

/// Processed image bytes ready for storage.
#[derive(Debug, Clone)]
pub(crate) struct NewStoredImage {
    /// Stable image identifier.
    pub(crate) id: Uuid,
    /// User that owns this image.
    pub(crate) owner_user_id: Uuid,
    /// Image purpose.
    pub(crate) kind: String,
    /// Stored MIME type.
    pub(crate) content_type: String,
    /// Pixel width.
    pub(crate) width: i32,
    /// Pixel height.
    pub(crate) height: i32,
    /// Stored byte size.
    pub(crate) byte_size: i64,
    /// Hex-encoded SHA-256 digest.
    pub(crate) sha256: String,
    /// Storage backend name.
    pub(crate) storage_backend: String,
    /// External object storage key.
    pub(crate) storage_key: Option<String>,
    /// Stored image bytes.
    pub(crate) data: Option<Vec<u8>>,
}

/// Stored image data.
#[derive(Debug, Clone)]
pub(crate) struct StoredImage {
    /// Stable image identifier.
    pub(crate) id: Uuid,
    /// User that owns this image.
    pub(crate) owner_user_id: Uuid,
    /// Image purpose.
    pub(crate) kind: String,
    /// Stored MIME type.
    pub(crate) content_type: String,
    /// Pixel width.
    pub(crate) width: i32,
    /// Pixel height.
    pub(crate) height: i32,
    /// Stored byte size.
    pub(crate) byte_size: i64,
    /// Storage backend name.
    pub(crate) storage_backend: String,
    /// Stored image bytes when database-backed.
    pub(crate) data: Option<Vec<u8>>,
}
