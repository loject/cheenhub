//! Server application tests.

use std::sync::Arc;

use cheenhub_contracts::rest::{
    CreateServerInviteRequest, CreateServerRequest, CreateServerRoomRequest, RegisterRequest,
    ServerRoomKind, UpdateServerRoomRequest,
};
use uuid::Uuid;

use super::{
    accept_invite, create, create_invite, create_room, delete_room, invite_info, leave, list,
    list_rooms, update_room,
};
use crate::features::auth::application as auth_application;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::servers::error::ServerError;
use crate::features::servers::infrastructure::{InMemoryServerStore, ServerStore};
use crate::features::text_chat::infrastructure::InMemoryTextChatStore;
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;

mod invite_errors;
mod invites;
mod rooms_and_list;

fn state() -> AppState {
    state_with_store(Arc::new(InMemoryServerStore::default()))
}

fn state_with_store(server_store: Arc<InMemoryServerStore>) -> AppState {
    AppState {
        auth_store: Arc::new(InMemoryAuthStore::default()),
        server_store,
        text_chat_store: Arc::new(InMemoryTextChatStore::default()),
        voice_presence_store: Arc::new(
            crate::features::voice_chat::infrastructure::InMemoryVoicePresenceStore::default(),
        ),
        realtime_hub: Arc::new(RealtimeHub::default()),
        auth_keys: AuthKeys::generate_for_tests(),
        access_token_lifetime_minutes: 15,
        refresh_token_lifetime_days: 30,
        google_oauth_client_id: Some("test-google-client".to_owned()),
        google_oauth_client_secret: Some("test-google-secret".to_owned()),
        google_oauth_redirect_uri: Some(
            "http://localhost/api/auth/oauth/google/callback".to_owned(),
        ),
        cheenhub_client_base_url: "http://localhost".to_owned(),
        oauth_state_lifetime_minutes: 10,
        oauth_handoff_lifetime_minutes: 5,
        oauth_registration_lifetime_minutes: 15,
    }
}
