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
    pub(crate) password_hash: Option<String>,
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

/// Linked OAuth account data.
#[derive(Debug, Clone)]
pub(crate) struct OAuthAccount {
    /// User that owns the linked account.
    pub(crate) user_id: Uuid,
    /// External OAuth provider.
    pub(crate) provider: String,
    /// Stable provider-side subject.
    pub(crate) provider_subject: String,
    /// Provider email address.
    pub(crate) email: String,
    /// Provider display name.
    pub(crate) display_name: Option<String>,
    /// Timestamp when the provider was linked.
    pub(crate) linked_at: DateTime<Utc>,
}

/// Short-lived OAuth state created before provider redirect.
#[derive(Debug, Clone)]
pub(crate) struct OAuthState {
    /// OAuth nonce sent to the provider.
    pub(crate) nonce: String,
    /// Flow kind.
    pub(crate) flow_kind: String,
    /// Authenticated user for link flow.
    pub(crate) user_id: Option<Uuid>,
}

/// Short-lived OAuth registration intent.
#[derive(Debug, Clone)]
pub(crate) struct OAuthRegistrationIntent {
    /// Stable intent row identifier.
    pub(crate) id: Uuid,
    /// Stable provider-side subject.
    pub(crate) provider_subject: String,
    /// Verified provider email address.
    pub(crate) email: String,
    /// Provider display name.
    pub(crate) display_name: Option<String>,
}

/// Short-lived OAuth frontend handoff.
#[derive(Debug, Clone)]
pub(crate) struct OAuthHandoff {
    /// Stable handoff row identifier.
    pub(crate) id: Uuid,
    /// Handoff result kind.
    pub(crate) kind: String,
    /// User id for authenticated and linked handoffs.
    pub(crate) user_id: Option<Uuid>,
    /// Registration intent id for registration handoffs.
    pub(crate) registration_intent_id: Option<Uuid>,
}
