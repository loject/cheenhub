//! Server domain models.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Server data used by server flows.
#[derive(Debug, Clone)]
pub(crate) struct Server {
    /// Stable server identifier.
    pub(crate) id: Uuid,
    /// User that owns the server.
    #[allow(dead_code)]
    pub(crate) owner_user_id: Uuid,
    /// Human-readable server name.
    pub(crate) name: String,
    /// Server creation timestamp.
    #[allow(dead_code)]
    pub(crate) created_at: DateTime<Utc>,
    /// Last server update timestamp.
    #[allow(dead_code)]
    pub(crate) updated_at: DateTime<Utc>,
}
