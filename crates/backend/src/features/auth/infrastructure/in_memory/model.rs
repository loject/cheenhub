//! In-memory authentication storage models.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::auth::domain::{OAuthAccount, OAuthRegistrationIntent, UserAccount};

/// In-memory auth store state.
#[derive(Default)]
pub(super) struct InMemoryState {
    /// User accounts.
    pub(super) users: Vec<InMemoryUser>,
    /// Login sessions.
    pub(super) sessions: Vec<InMemorySession>,
    /// Refresh tokens.
    pub(super) refresh_tokens: Vec<InMemoryRefreshToken>,
    /// Linked OAuth accounts.
    pub(super) oauth_accounts: Vec<OAuthAccount>,
    /// OAuth authorization states.
    pub(super) oauth_states: Vec<InMemoryOAuthState>,
    /// OAuth frontend handoffs.
    pub(super) oauth_handoffs: Vec<InMemoryOAuthHandoff>,
    /// OAuth registration intents.
    pub(super) oauth_registration_intents: Vec<InMemoryOAuthRegistrationIntent>,
}

/// In-memory user row.
#[derive(Debug, Clone)]
pub(super) struct InMemoryUser {
    /// User account.
    pub(super) account: UserAccount,
    /// Normalized email for lookup.
    pub(super) email_normalized: String,
}

/// In-memory session row.
#[derive(Debug, Clone)]
pub(super) struct InMemorySession {
    /// Session id.
    pub(super) id: Uuid,
    /// Owner user id.
    pub(super) user_id: Uuid,
    /// Expiration timestamp.
    pub(super) expires_at: DateTime<Utc>,
    /// Revocation timestamp.
    pub(super) revoked_at: Option<DateTime<Utc>>,
}

/// In-memory refresh token row.
#[derive(Debug, Clone)]
pub(super) struct InMemoryRefreshToken {
    /// Refresh token row id.
    pub(super) id: Uuid,
    /// Owning session id.
    pub(super) session_id: Uuid,
    /// Token hash.
    pub(super) token_hash: String,
    /// Expiration timestamp.
    pub(super) expires_at: DateTime<Utc>,
    /// Revocation timestamp.
    pub(super) revoked_at: Option<DateTime<Utc>>,
}

/// In-memory OAuth state row.
#[derive(Debug, Clone)]
pub(super) struct InMemoryOAuthState {
    /// State hash.
    pub(super) state_hash: String,
    /// OAuth nonce.
    pub(super) nonce: String,
    /// Flow kind.
    pub(super) flow_kind: String,
    /// Link flow user id.
    pub(super) user_id: Option<Uuid>,
    /// Expiration timestamp.
    pub(super) expires_at: DateTime<Utc>,
    /// Consumption timestamp.
    pub(super) consumed_at: Option<DateTime<Utc>>,
}

/// In-memory OAuth handoff row.
#[derive(Debug, Clone)]
pub(super) struct InMemoryOAuthHandoff {
    /// Handoff row id.
    pub(super) id: Uuid,
    /// Handoff code hash.
    pub(super) code_hash: String,
    /// Handoff kind.
    pub(super) kind: String,
    /// User id.
    pub(super) user_id: Option<Uuid>,
    /// Registration intent id.
    pub(super) registration_intent_id: Option<Uuid>,
    /// Expiration timestamp.
    pub(super) expires_at: DateTime<Utc>,
    /// Consumption timestamp.
    pub(super) consumed_at: Option<DateTime<Utc>>,
}

/// In-memory OAuth registration intent row.
#[derive(Debug, Clone)]
pub(super) struct InMemoryOAuthRegistrationIntent {
    /// Registration intent.
    pub(super) intent: OAuthRegistrationIntent,
    /// OAuth provider.
    pub(super) provider: String,
    /// Expiration timestamp.
    pub(super) expires_at: DateTime<Utc>,
    /// Consumption timestamp.
    pub(super) consumed_at: Option<DateTime<Utc>>,
}
