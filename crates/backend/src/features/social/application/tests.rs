use std::sync::Arc;

use axum::body::Bytes;
use image::{ImageBuffer, ImageFormat, Rgba};

use cheenhub_contracts::rest::{
    DmMessageDeliveryStatus, MarkDmConversationReadRequest, OpenDmConversationRequest,
    RegisterRequest, SendDmMessageRequest, SendFriendRequestRequest,
};

use super::{
    accept_friend_request, dm_image, list_dm_conversations, list_dm_messages, list_friends,
    mark_dm_conversation_read, open_dm_conversation, send_dm_message, send_friend_request,
    upload_dm_image,
};
use crate::features::auth::application as auth_application;
use crate::features::auth::email::tests::TestAuthMailer;
use crate::features::auth::infrastructure::InMemoryAuthStore;
use crate::features::auth::security::keys::AuthKeys;
use crate::features::images::infrastructure::InMemoryImageStore;
use crate::features::servers::infrastructure::InMemoryServerStore;
use crate::features::social::infrastructure::InMemorySocialStore;
use crate::features::text_chat::infrastructure::{
    InMemoryChatAttachmentObjectStore, InMemoryTextChatStore,
};
use crate::features::voice_chat::infrastructure::InMemoryVoicePresenceStore;
use crate::realtime::hub::RealtimeHub;
use crate::state::AppState;

#[tokio::test]
async fn incoming_direct_message_increments_unread_count() {
    let setup = setup_pair().await;

    send_dm_message(
        &setup.state,
        &setup.bob_access_token,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            image_id: None,
            body: "Привет".to_owned(),
        },
    )
    .await
    .expect("message should send");

    let alice_conversations = list_dm_conversations(&setup.state, &setup.alice_access_token)
        .await
        .expect("conversations should load");
    let alice_conversation = &alice_conversations.conversations[0];
    assert_eq!(alice_conversation.unread_count, 1);

    let bob_conversations = list_dm_conversations(&setup.state, &setup.bob_access_token)
        .await
        .expect("conversations should load");
    assert_eq!(bob_conversations.conversations[0].unread_count, 0);

    let alice_friends = list_friends(&setup.state, &setup.alice_access_token)
        .await
        .expect("friends should load");
    assert_eq!(alice_friends.friends[0].unread_count, 1);
}

#[tokio::test]
async fn mark_as_read_advances_seq_once_and_counts_only_incoming_messages() {
    let setup = setup_pair().await;
    let bob_first = send_dm_message(
        &setup.state,
        &setup.bob_access_token,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            image_id: None,
            body: "Первое входящее".to_owned(),
        },
    )
    .await
    .expect("first incoming message should send")
    .message;
    let alice_own = send_dm_message(
        &setup.state,
        &setup.alice_access_token,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            image_id: None,
            body: "Мой ответ".to_owned(),
        },
    )
    .await
    .expect("own message should send")
    .message;
    let bob_second = send_dm_message(
        &setup.state,
        &setup.bob_access_token,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            image_id: None,
            body: "Второе входящее".to_owned(),
        },
    )
    .await
    .expect("second incoming message should send")
    .message;

    assert_eq!(conversation_unread(&setup).await, 2);

    let first_read = mark_dm_conversation_read(
        &setup.state,
        &setup.alice_access_token,
        setup.conversation_id.clone(),
        MarkDmConversationReadRequest {
            last_read_message_id: alice_own.id.clone(),
        },
    )
    .await
    .expect("mark read should succeed");
    assert_eq!(first_read.last_read_seq, alice_own.seq);
    assert_eq!(first_read.conversation_unread_count, 1);

    let repeated = mark_dm_conversation_read(
        &setup.state,
        &setup.alice_access_token,
        setup.conversation_id.clone(),
        MarkDmConversationReadRequest {
            last_read_message_id: alice_own.id.clone(),
        },
    )
    .await
    .expect("repeated mark read should be idempotent");
    assert_eq!(repeated.last_read_seq, alice_own.seq);
    assert_eq!(repeated.conversation_unread_count, 1);

    let stale = mark_dm_conversation_read(
        &setup.state,
        &setup.alice_access_token,
        setup.conversation_id.clone(),
        MarkDmConversationReadRequest {
            last_read_message_id: bob_first.id.clone(),
        },
    )
    .await
    .expect("stale mark read should be ignored");
    assert_eq!(stale.last_read_seq, alice_own.seq);
    assert_eq!(stale.conversation_unread_count, 1);

    let final_read = mark_dm_conversation_read(
        &setup.state,
        &setup.alice_access_token,
        setup.conversation_id.clone(),
        MarkDmConversationReadRequest {
            last_read_message_id: bob_second.id.clone(),
        },
    )
    .await
    .expect("new mark read should advance");
    assert_eq!(final_read.last_read_seq, bob_second.seq);
    assert_eq!(final_read.conversation_unread_count, 0);

    let conversation_id = uuid::Uuid::parse_str(&setup.conversation_id).expect("valid id");
    let alice_id = uuid::Uuid::parse_str(&setup.alice_user_id).expect("valid id");
    let first_read_at = setup
        .state
        .social_store
        .message_read_at(&conversation_id, &alice_id, bob_first.seq)
        .await
        .expect("read lookup should succeed");
    let second_read_at = setup
        .state
        .social_store
        .message_read_at(&conversation_id, &alice_id, bob_second.seq)
        .await
        .expect("read lookup should succeed");
    assert!(first_read_at.is_some());
    assert!(second_read_at.is_some());
    assert!(first_read_at <= second_read_at);
}

#[tokio::test]
async fn outgoing_direct_message_is_read_when_recipient_read_seq_reaches_it() {
    let setup = setup_pair().await;
    let alice_message = send_dm_message(
        &setup.state,
        &setup.alice_access_token,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            image_id: None,
            body: "Проверка галочек".to_owned(),
        },
    )
    .await
    .expect("message should send")
    .message;

    let before_read = list_dm_messages(
        &setup.state,
        &setup.alice_access_token,
        setup.conversation_id.clone(),
        None,
    )
    .await
    .expect("messages should load");
    assert_eq!(
        before_read.messages[0].delivery_status,
        Some(DmMessageDeliveryStatus::Accepted)
    );

    mark_dm_conversation_read(
        &setup.state,
        &setup.bob_access_token,
        setup.conversation_id.clone(),
        MarkDmConversationReadRequest {
            last_read_message_id: alice_message.id,
        },
    )
    .await
    .expect("recipient should mark message read");

    let after_read = list_dm_messages(
        &setup.state,
        &setup.alice_access_token,
        setup.conversation_id,
        None,
    )
    .await
    .expect("messages should load");
    assert_eq!(
        after_read.messages[0].delivery_status,
        Some(DmMessageDeliveryStatus::Read)
    );
}

#[tokio::test]
async fn direct_message_image_upload_send_and_load_is_scoped_and_single_use() {
    let setup = setup_pair().await;
    let bytes = test_png();
    let uploaded = upload_dm_image(
        &setup.state,
        &setup.alice_access_token,
        setup.conversation_id.clone(),
        Bytes::from(bytes.clone()),
    )
    .await
    .expect("image should upload")
    .image;

    let orphan_error = dm_image(
        &setup.state,
        &setup.alice_access_token,
        setup.conversation_id.clone(),
        uploaded.id.clone(),
    )
    .await
    .expect_err("unattached image should not be readable");
    assert!(matches!(
        orphan_error,
        crate::features::social::SocialError::NotFound(_)
    ));

    let sent = send_dm_message(
        &setup.state,
        &setup.alice_access_token,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: String::new(),
            image_id: Some(uploaded.id.clone()),
        },
    )
    .await
    .expect("image message should send")
    .message;
    assert!(sent.body.is_empty());
    assert_eq!(
        sent.image.as_ref().map(|image| image.id.as_str()),
        Some(uploaded.id.as_str())
    );
    assert_eq!(
        crate::features::push_notifications::direct_message_preview(&sent.body, true),
        "Изображение"
    );

    let loaded = dm_image(
        &setup.state,
        &setup.bob_access_token,
        setup.conversation_id.clone(),
        uploaded.id.clone(),
    )
    .await
    .expect("conversation participant should load attached image");
    assert_eq!(loaded.data.as_deref(), Some(bytes.as_slice()));

    let reused = send_dm_message(
        &setup.state,
        &setup.alice_access_token,
        setup.conversation_id,
        SendDmMessageRequest {
            body: String::new(),
            image_id: Some(uploaded.id),
        },
    )
    .await
    .expect_err("attached image must not be reusable");
    assert!(matches!(
        reused,
        crate::features::social::SocialError::BadRequest(_)
    ));
}

#[tokio::test]
async fn direct_message_image_is_hidden_from_unrelated_conversation_member() {
    let setup = setup_pair().await;
    let charlie = registered_user(&setup.state, "charlie_dm", "charlie-dm@example.com").await;
    let dave = registered_user(&setup.state, "dave_dm", "dave-dm@example.com").await;
    let request = send_friend_request(
        &setup.state,
        &charlie.access_token,
        SendFriendRequestRequest {
            recipient_user_id: dave.user.id.clone(),
        },
    )
    .await
    .expect("second pair request should send");
    accept_friend_request(&setup.state, &dave.access_token, request.request.id)
        .await
        .expect("second pair request should accept");
    let foreign_conversation = open_dm_conversation(
        &setup.state,
        &charlie.access_token,
        OpenDmConversationRequest {
            friend_user_id: dave.user.id.clone(),
        },
    )
    .await
    .expect("second pair conversation should open")
    .conversation;
    let uploaded = upload_dm_image(
        &setup.state,
        &charlie.access_token,
        foreign_conversation.id.clone(),
        Bytes::from(test_png()),
    )
    .await
    .expect("foreign image should upload")
    .image;
    send_dm_message(
        &setup.state,
        &charlie.access_token,
        foreign_conversation.id.clone(),
        SendDmMessageRequest {
            body: String::new(),
            image_id: Some(uploaded.id.clone()),
        },
    )
    .await
    .expect("foreign image should attach");

    let denied = dm_image(
        &setup.state,
        &setup.alice_access_token,
        foreign_conversation.id,
        uploaded.id,
    )
    .await
    .expect_err("unrelated user must not load image");
    assert!(matches!(
        denied,
        crate::features::social::SocialError::NotFound(_)
    ));
}

async fn conversation_unread(setup: &PairSetup) -> i64 {
    list_dm_conversations(&setup.state, &setup.alice_access_token)
        .await
        .expect("conversations should load")
        .conversations[0]
        .unread_count
}

async fn setup_pair() -> PairSetup {
    let state = state();
    let alice = registered_user(&state, "alice_dm", "alice-dm@example.com").await;
    let bob = registered_user(&state, "bob_dm", "bob-dm@example.com").await;

    let request = send_friend_request(
        &state,
        &alice.access_token,
        SendFriendRequestRequest {
            recipient_user_id: bob.user.id.clone(),
        },
    )
    .await
    .expect("friend request should send");
    accept_friend_request(&state, &bob.access_token, request.request.id)
        .await
        .expect("friend request should accept");
    let conversation = open_dm_conversation(
        &state,
        &alice.access_token,
        OpenDmConversationRequest {
            friend_user_id: bob.user.id.clone(),
        },
    )
    .await
    .expect("conversation should open")
    .conversation;

    PairSetup {
        state,
        alice_access_token: alice.access_token,
        bob_access_token: bob.access_token,
        alice_user_id: alice.user.id,
        conversation_id: conversation.id,
    }
}

async fn registered_user(
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

fn state() -> AppState {
    AppState {
        auth_store: Arc::new(InMemoryAuthStore::default()),
        auth_mailer: Arc::new(TestAuthMailer::default()),
        server_store: Arc::new(InMemoryServerStore::default()),
        social_store: Arc::new(InMemorySocialStore::default()),
        text_chat_store: Arc::new(InMemoryTextChatStore::default()),
        chat_attachment_object_store: Arc::new(InMemoryChatAttachmentObjectStore::new(
            "test-chat-images",
        )),
        image_store: Arc::new(InMemoryImageStore::default()),
        push_notifications: Arc::new(
            crate::features::push_notifications::application::PushNotifications::disabled(
                Arc::new(InMemoryAuthStore::default()),
            ),
        ),
        image_processing_queue: Arc::new(tokio::sync::Semaphore::new(1)),
        voice_presence_store: Arc::new(InMemoryVoicePresenceStore::default()),
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

fn test_png() -> Vec<u8> {
    let image = ImageBuffer::from_pixel(2, 2, Rgba([40_u8, 120, 220, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Png)
        .expect("test image should encode");
    bytes.into_inner()
}

struct PairSetup {
    state: AppState,
    alice_access_token: String,
    bob_access_token: String,
    alice_user_id: String,
    conversation_id: String,
}
