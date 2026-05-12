//! Shared backend application state.

use std::sync::Arc;

use crate::features::auth::infrastructure::AuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::servers::infrastructure::ServerStore;
use crate::features::text_chat::infrastructure::TextChatStore;
use crate::features::voice_chat::infrastructure::InMemoryVoicePresenceStore;
use crate::realtime::hub::RealtimeHub;

/// Shared backend application state.
#[derive(Clone)]
pub(crate) struct AppState {
    /// Authentication storage backend.
    pub(crate) auth_store: Arc<dyn AuthStore>,
    /// Server storage backend.
    pub(crate) server_store: Arc<dyn ServerStore>,
    /// Text chat storage backend.
    pub(crate) text_chat_store: Arc<dyn TextChatStore>,
    /// Active voice room presence.
    pub(crate) voice_presence_store: Arc<InMemoryVoicePresenceStore>,
    /// Shared realtime stream registry and fanout hub.
    pub(crate) realtime_hub: Arc<RealtimeHub>,
    /// Access JWT signing keys.
    pub(crate) auth_keys: AuthKeys,
    /// Access JWT lifetime in minutes.
    pub(crate) access_token_lifetime_minutes: i64,
    /// Refresh token lifetime in days.
    pub(crate) refresh_token_lifetime_days: i64,
    /// Google OAuth client id.
    pub(crate) google_oauth_client_id: Option<String>,
    /// Google OAuth client secret.
    pub(crate) google_oauth_client_secret: Option<String>,
    /// Google OAuth redirect URI registered for this backend.
    pub(crate) google_oauth_redirect_uri: Option<String>,
    /// Browser client base URL used after OAuth callbacks.
    pub(crate) cheenhub_client_base_url: String,
    /// OAuth state lifetime in minutes.
    pub(crate) oauth_state_lifetime_minutes: i64,
    /// OAuth handoff lifetime in minutes.
    pub(crate) oauth_handoff_lifetime_minutes: i64,
    /// OAuth registration intent lifetime in minutes.
    pub(crate) oauth_registration_lifetime_minutes: i64,
}
