//! In-memory authentication storage models.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::features::auth::domain::{OAuthAccount, OAuthRegistrationIntent, UserAccount};

/// In-memory auth store state.
#[derive(Default)]
pub(in crate::features::auth::infrastructure) struct InMemoryState {
    /// User accounts.
    pub(in crate::features::auth::infrastructure) users: Vec<InMemoryUser>,
    /// Login sessions.
    pub(in crate::features::auth::infrastructure) sessions: Vec<InMemorySession>,
    /// Refresh tokens.
    pub(in crate::features::auth::infrastructure) refresh_tokens: Vec<InMemoryRefreshToken>,
    /// Linked OAuth accounts.
    pub(in crate::features::auth::infrastructure) oauth_accounts: Vec<OAuthAccount>,
    /// OAuth authorization states.
    pub(in crate::features::auth::infrastructure) oauth_states: Vec<InMemoryOAuthState>,
    /// OAuth frontend handoffs.
    pub(in crate::features::auth::infrastructure) oauth_handoffs: Vec<InMemoryOAuthHandoff>,
    /// OAuth registration intents.
    pub(in crate::features::auth::infrastructure) oauth_registration_intents:
        Vec<InMemoryOAuthRegistrationIntent>,
    /// Password reset tokens.
    pub(in crate::features::auth::infrastructure) password_reset_tokens:
        Vec<InMemoryPasswordResetToken>,
    /// User nickname change history.
    pub(in crate::features::auth::infrastructure) user_nickname_history:
        Vec<(Uuid, Uuid, Uuid, String, String, DateTime<Utc>)>,
}

/// In-memory user row.
#[derive(Debug, Clone)]
pub(in crate::features::auth::infrastructure) struct InMemoryUser {
    /// User account.
    pub(in crate::features::auth::infrastructure) account: UserAccount,
    /// Normalized email for lookup.
    pub(in crate::features::auth::infrastructure) email_normalized: String,
}

/// In-memory session row.
#[derive(Debug, Clone)]
pub(in crate::features::auth::infrastructure) struct InMemorySession {
    /// Session id.
    pub(in crate::features::auth::infrastructure) id: Uuid,
    /// Owner user id.
    pub(in crate::features::auth::infrastructure) user_id: Uuid,
    /// Expiration timestamp.
    pub(in crate::features::auth::infrastructure) expires_at: DateTime<Utc>,
    /// Revocation timestamp.
    pub(in crate::features::auth::infrastructure) revoked_at: Option<DateTime<Utc>>,
}

/// In-memory refresh token row.
#[derive(Debug, Clone)]
pub(in crate::features::auth::infrastructure) struct InMemoryRefreshToken {
    /// Refresh token row id.
    pub(in crate::features::auth::infrastructure) id: Uuid,
    /// Owning session id.
    pub(in crate::features::auth::infrastructure) session_id: Uuid,
    /// Token hash.
    pub(in crate::features::auth::infrastructure) token_hash: String,
    /// Expiration timestamp.
    pub(in crate::features::auth::infrastructure) expires_at: DateTime<Utc>,
    /// Revocation timestamp.
    pub(in crate::features::auth::infrastructure) revoked_at: Option<DateTime<Utc>>,
}

/// In-memory OAuth state row.
#[derive(Debug, Clone)]
pub(in crate::features::auth::infrastructure) struct InMemoryOAuthState {
    /// State hash.
    pub(in crate::features::auth::infrastructure) state_hash: String,
    /// OAuth nonce.
    pub(in crate::features::auth::infrastructure) nonce: String,
    /// Flow kind.
    pub(in crate::features::auth::infrastructure) flow_kind: String,
    /// Link flow user id.
    pub(in crate::features::auth::infrastructure) user_id: Option<Uuid>,
    /// Expiration timestamp.
    pub(in crate::features::auth::infrastructure) expires_at: DateTime<Utc>,
    /// Consumption timestamp.
    pub(in crate::features::auth::infrastructure) consumed_at: Option<DateTime<Utc>>,
}

/// In-memory OAuth handoff row.
#[derive(Debug, Clone)]
pub(in crate::features::auth::infrastructure) struct InMemoryOAuthHandoff {
    /// Handoff row id.
    pub(in crate::features::auth::infrastructure) id: Uuid,
    /// Handoff code hash.
    pub(in crate::features::auth::infrastructure) code_hash: String,
    /// Handoff kind.
    pub(in crate::features::auth::infrastructure) kind: String,
    /// User id.
    pub(in crate::features::auth::infrastructure) user_id: Option<Uuid>,
    /// Registration intent id.
    pub(in crate::features::auth::infrastructure) registration_intent_id: Option<Uuid>,
    /// Expiration timestamp.
    pub(in crate::features::auth::infrastructure) expires_at: DateTime<Utc>,
    /// Consumption timestamp.
    pub(in crate::features::auth::infrastructure) consumed_at: Option<DateTime<Utc>>,
}

/// In-memory OAuth registration intent row.
#[derive(Debug, Clone)]
pub(in crate::features::auth::infrastructure) struct InMemoryOAuthRegistrationIntent {
    /// Registration intent.
    pub(in crate::features::auth::infrastructure) intent: OAuthRegistrationIntent,
    /// OAuth provider.
    pub(in crate::features::auth::infrastructure) provider: String,
    /// Expiration timestamp.
    pub(in crate::features::auth::infrastructure) expires_at: DateTime<Utc>,
    /// Consumption timestamp.
    pub(in crate::features::auth::infrastructure) consumed_at: Option<DateTime<Utc>>,
}

/// In-memory password reset token row.
#[derive(Debug, Clone)]
pub(in crate::features::auth::infrastructure) struct InMemoryPasswordResetToken {
    /// Reset token row id.
    pub(in crate::features::auth::infrastructure) id: Uuid,
    /// Owner user id.
    pub(in crate::features::auth::infrastructure) user_id: Uuid,
    /// Reset token hash.
    pub(in crate::features::auth::infrastructure) token_hash: String,
    /// Expiration timestamp.
    pub(in crate::features::auth::infrastructure) expires_at: DateTime<Utc>,
    /// Consumption timestamp.
    pub(in crate::features::auth::infrastructure) consumed_at: Option<DateTime<Utc>>,
}
