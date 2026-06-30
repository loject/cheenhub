//! Server application tests.

use std::sync::Arc;

use bytes::Bytes;
use cheenhub_contracts::rest::{
    CreateServerInviteRequest, CreateServerRequest, CreateServerRoomRequest, RegisterRequest,
    ServerRoomKind, UpdateServerRequest, UpdateServerRoomRequest,
};
use uuid::Uuid;

use super::{
    accept_invite, assign_server_member_role, create, create_invite, create_room, delete_room,
    invite_info, kick_server_invite_member, kick_server_member, leave, list, list_rooms,
    list_server_invites, list_server_members, list_server_roles, revoke_server_invite,
    save_server_roles, update, update_avatar, update_room,
};
use crate::features::auth::application as auth_application;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::servers::error::ServerError;
use crate::features::servers::infrastructure::{InMemoryServerStore, ServerStore};
use crate::features::social::infrastructure::InMemorySocialStore;
use crate::features::text_chat::infrastructure::InMemoryTextChatStore;
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;

mod invite_errors;
mod invite_permissions;
mod invite_settings;
mod invites;
mod members_settings;
mod rooms_and_list;
mod server_profile;

fn state() -> AppState {
    state_with_store(Arc::new(InMemoryServerStore::default()))
}

fn state_with_store(server_store: Arc<InMemoryServerStore>) -> AppState {
    AppState {
        auth_store: Arc::new(InMemoryAuthStore::default()),
        auth_mailer: Arc::new(crate::features::auth::email::tests::TestAuthMailer::default()),
        server_store,
        social_store: Arc::new(InMemorySocialStore::default()),
        text_chat_store: Arc::new(InMemoryTextChatStore::default()),
        chat_attachment_object_store: Arc::new(
            crate::features::text_chat::infrastructure::InMemoryChatAttachmentObjectStore::new(
                "test-chat-images",
            ),
        ),
        image_store: Arc::new(
            crate::features::images::infrastructure::InMemoryImageStore::default(),
        ),
        image_processing_queue: Arc::new(tokio::sync::Semaphore::new(1)),
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
        cheenhub_api_base_url: "http://localhost/api".to_owned(),
        oauth_state_lifetime_minutes: 10,
        oauth_handoff_lifetime_minutes: 5,
        oauth_registration_lifetime_minutes: 15,
        password_reset_token_lifetime_minutes: 30,
    }
}
