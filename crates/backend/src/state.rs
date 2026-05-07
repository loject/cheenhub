//! Shared backend application state.

use std::sync::Arc;

use crate::features::auth::infrastructure::AuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::servers::infrastructure::ServerStore;
use crate::features::text_chat::infrastructure::TextChatStore;
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
    /// Shared realtime stream registry and fanout hub.
    pub(crate) realtime_hub: Arc<RealtimeHub>,
    /// Access JWT signing keys.
    pub(crate) auth_keys: AuthKeys,
    /// Access JWT lifetime in minutes.
    pub(crate) access_token_lifetime_minutes: i64,
    /// Refresh token lifetime in days.
    pub(crate) refresh_token_lifetime_days: i64,
}
