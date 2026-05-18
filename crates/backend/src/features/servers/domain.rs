//! Server domain models.

use cheenhub_contracts::rest::ServerRoomKind;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Server data used by server flows.
#[derive(Debug, Clone)]
pub(crate) struct Server {
    /// Stable server identifier.
    pub(crate) id: Uuid,
    /// User that owns the server.
    pub(crate) owner_user_id: Uuid,
    /// Human-readable server name.
    pub(crate) name: String,
    /// Stored server avatar image identifier.
    pub(crate) avatar_image_id: Option<Uuid>,
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

/// Server room data used by server flows.
#[derive(Debug, Clone)]
pub(crate) struct ServerRoom {
    /// Stable room identifier.
    pub(crate) id: Uuid,
    /// Server the room belongs to.
    pub(crate) server_id: Uuid,
    /// Human-readable room name.
    pub(crate) name: String,
    /// Room interaction type.
    pub(crate) kind: ServerRoomKind,
    /// Append-only ordering position inside the server.
    pub(crate) position: u32,
    /// Room creation timestamp.
    #[allow(dead_code)]
    pub(crate) created_at: DateTime<Utc>,
    /// Last room update timestamp.
    pub(crate) updated_at: DateTime<Utc>,
}

/// Server invite data used by server flows.
#[derive(Debug, Clone)]
pub(crate) struct ServerInvite {
    /// Stable invite identifier used as the invite code.
    pub(crate) id: Uuid,
    /// Server the invite belongs to.
    pub(crate) server_id: Uuid,
    /// User that created the invite.
    pub(crate) creator_user_id: Uuid,
    /// Optional maximum number of accepted invite uses.
    pub(crate) max_uses: Option<u32>,
    /// Optional invite expiration timestamp.
    pub(crate) expires_at: Option<DateTime<Utc>>,
    /// Invite creation timestamp.
    pub(crate) created_at: DateTime<Utc>,
    /// Invite revocation timestamp.
    pub(crate) revoked_at: Option<DateTime<Utc>>,
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

/// Temporary server exclusion that blocks a kicked user from rejoining.
#[derive(Debug, Clone)]
pub(crate) struct ServerMemberExclusion {
    /// Stable exclusion row identifier.
    #[allow(dead_code)]
    pub(crate) id: Uuid,
    /// Server the exclusion belongs to.
    pub(crate) server_id: Uuid,
    /// User blocked from rejoining.
    pub(crate) user_id: Uuid,
    /// User or system actor that created the exclusion.
    #[allow(dead_code)]
    pub(crate) initiator_user_id: Uuid,
    /// Timestamp until which the user cannot rejoin.
    pub(crate) expires_at: DateTime<Utc>,
    /// Exclusion creation timestamp.
    #[allow(dead_code)]
    pub(crate) created_at: DateTime<Utc>,
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
    pub(crate) user_id: Uuid,
    /// Invite use timestamp.
    pub(crate) used_at: DateTime<Utc>,
}
