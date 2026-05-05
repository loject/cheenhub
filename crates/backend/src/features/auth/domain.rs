//! Authentication domain models.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// User account data used by authentication flows.
#[derive(Debug, Clone)]
pub(crate) struct UserAccount {
    /// Stable user identifier.
    pub(crate) id: Uuid,
    /// Public nickname shown to other users.
    pub(crate) nickname: String,
    /// Email address used for login.
    pub(crate) email: String,
    /// Stored Argon2 password hash.
    pub(crate) password_hash: String,
    /// Account registration timestamp.
    pub(crate) registered_at: DateTime<Utc>,
}

/// Active refresh token session with its owning user.
#[derive(Debug, Clone)]
pub(crate) struct RefreshSession {
    /// Refresh token row identifier.
    pub(crate) refresh_token_id: Uuid,
    /// Session row identifier.
    pub(crate) session_id: Uuid,
    /// User that owns the session.
    pub(crate) user: UserAccount,
}
