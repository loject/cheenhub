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

/// Server invite data used by server flows.
#[derive(Debug, Clone)]
pub(crate) struct ServerInvite {
    /// Stable invite identifier used as the invite code.
    pub(crate) id: Uuid,
    /// Server the invite belongs to.
    #[allow(dead_code)]
    pub(crate) server_id: Uuid,
    /// User that created the invite.
    #[allow(dead_code)]
    pub(crate) creator_user_id: Uuid,
    /// Optional maximum number of accepted invite uses.
    #[allow(dead_code)]
    pub(crate) max_uses: Option<u32>,
    /// Optional invite expiration timestamp.
    #[allow(dead_code)]
    pub(crate) expires_at: Option<DateTime<Utc>>,
    /// Invite creation timestamp.
    #[allow(dead_code)]
    pub(crate) created_at: DateTime<Utc>,
}
