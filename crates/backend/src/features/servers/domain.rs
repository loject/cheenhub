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

/// Server data with current-user membership context.
#[derive(Debug, Clone)]
pub(crate) struct ServerAccess {
    /// Server available to the current user.
    pub(crate) server: Server,
    /// Whether the current user is an active server member.
    pub(crate) is_member: bool,
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

/// Server member data used by server flows.
#[derive(Debug, Clone)]
pub(crate) struct ServerMember {
    /// Stable server member row identifier.
    #[allow(dead_code)]
    pub(crate) id: Uuid,
    /// Server the member belongs to.
    pub(crate) server_id: Uuid,
    /// User that joined the server.
    pub(crate) user_id: Uuid,
    /// Membership start timestamp.
    #[allow(dead_code)]
    pub(crate) joined_at: DateTime<Utc>,
    /// Membership end timestamp for future soft leave.
    pub(crate) left_at: Option<DateTime<Utc>>,
}

/// Server invite use data used by server flows.
#[derive(Debug, Clone)]
pub(crate) struct ServerInviteUse {
    /// Stable invite use row identifier.
    #[allow(dead_code)]
    pub(crate) id: Uuid,
    /// Invite that was used successfully.
    pub(crate) invite_id: Uuid,
    /// User that used the invite successfully.
    #[allow(dead_code)]
    pub(crate) user_id: Uuid,
    /// Invite use timestamp.
    #[allow(dead_code)]
    pub(crate) used_at: DateTime<Utc>,
}
