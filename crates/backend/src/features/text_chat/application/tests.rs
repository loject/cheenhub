use std::sync::Arc;

use cheenhub_contracts::rest::{RegisterRequest, ServerRoomKind};
use image::ImageEncoder;
use image::codecs::png::PngEncoder;
use uuid::Uuid;

use crate::features::auth::application as auth_application;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::social::infrastructure::InMemorySocialStore;
use crate::features::text_chat::infrastructure::InMemoryTextChatStore;
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;

mod attachments;
mod deletion;
mod history;
mod messages;

pub(super) fn state() -> AppState {
    AppState {
        auth_store: Arc::new(InMemoryAuthStore::default()),
        auth_mailer: Arc::new(crate::features::auth::email::tests::TestAuthMailer::default()),
        server_store: Arc::new(InMemoryServerStore::default()),
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
        push_notifications: Arc::new(
            crate::features::push_notifications::application::PushNotifications::disabled(
                Arc::new(InMemoryAuthStore::default()),
            ),
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

pub(super) async fn registered_user(
    state: &AppState,
    nickname: &str,
    email: &str,
) -> cheenhub_contracts::rest::AuthResponse {
    auth_application::register(
        state,
        RegisterRequest {
            nickname: nickname.to_owned(),
            email: email.to_owned(),
            password: "password123".to_owned(),
            accepts_policies: true,
        },
    )
    .await
    .expect("registration should succeed")
}

pub(super) async fn create_server_room(
    state: &AppState,
    owner_user_id: &Uuid,
    server_name: &str,
    room_name: &str,
    room_kind: ServerRoomKind,
) -> (String, String) {
    let server = state
        .server_store
        .insert_server(owner_user_id, server_name.to_owned())
        .await
        .expect("server should insert");
    state
        .server_store
        .insert_server_member(&server.id, owner_user_id)
        .await
        .expect("member should insert");
    let room = state
        .server_store
        .insert_server_room(&server.id, room_name.to_owned(), room_kind)
        .await
        .expect("room should insert");

    (server.id.to_string(), room.id.to_string())
}

pub(super) fn tiny_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(&[255, 255, 255, 255], 1, 1, image::ExtendedColorType::Rgba8)
        .expect("test png should encode");
    bytes
}
