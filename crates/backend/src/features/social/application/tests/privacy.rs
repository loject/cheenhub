//! Сценарии серверных ограничений контактов и блокировки пользователей.

use super::super::{
    block_user, edit_dm_message, ensure_incoming_call_allowed, ensure_incoming_room_invite_allowed,
    get_privacy_settings, list_blocked_users, unblock_user, update_privacy_settings,
};
use super::*;
use crate::features::social::SocialError;
use cheenhub_contracts::rest::{
    DirectMessageAudience, EditDmMessageRequest, IncomingContactAudience, UserPrivacySettings,
};

#[tokio::test]
async fn default_settings_allow_stranger_messages_but_require_friendship_for_calls_and_invites() {
    let state = state();
    let alice = registered_user(
        &state,
        "alice_privacy_default",
        "alice-privacy-default@example.com",
    )
    .await;
    let bob = registered_user(
        &state,
        "bob_privacy_default",
        "bob-privacy-default@example.com",
    )
    .await;
    let alice_id = user_id(&alice.user.id);
    let bob_id = user_id(&bob.user.id);

    let settings = get_privacy_settings(&state, &alice_id).await.unwrap();
    assert_eq!(settings, UserPrivacySettings::default());
    let conversation = open_dm_conversation(
        &state,
        &bob_id,
        OpenDmConversationRequest {
            friend_user_id: alice.user.id.clone(),
        },
    )
    .await
    .expect("stranger can start a conversation")
    .conversation;
    send_dm_message(
        &state,
        &bob_id,
        conversation.id,
        SendDmMessageRequest {
            body: "Привет".into(),
            image_id: None,
        },
    )
    .await
    .expect("stranger message is allowed by default");

    assert!(matches!(
        ensure_incoming_call_allowed(&state, &bob_id, &alice_id).await,
        Err(SocialError::Unauthorized(_))
    ));
    assert!(matches!(
        ensure_incoming_room_invite_allowed(&state, &bob_id, &alice_id).await,
        Err(SocialError::Unauthorized(_))
    ));
}

#[tokio::test]
async fn friends_only_dm_requires_new_recipient_message_and_restriction_reset_revokes_exception() {
    let setup = setup_pair().await;
    let alice_id = user_id(&setup.alice_user_id);
    let bob_id = user_id(&setup.bob_user_id);
    super::super::delete_friend(
        &setup.state,
        &setup.alice_access_token,
        setup.bob_user_id.clone(),
    )
    .await
    .unwrap();
    send_dm_message(
        &setup.state,
        &alice_id,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Старый текст".into(),
            image_id: None,
        },
    )
    .await
    .unwrap();

    update_privacy_settings(
        &setup.state,
        &alice_id,
        UserPrivacySettings {
            direct_messages: DirectMessageAudience::Friends,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let denied = send_dm_message(
        &setup.state,
        &bob_id,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Нельзя ответить на старое".into(),
            image_id: None,
        },
    )
    .await;
    assert!(matches!(denied, Err(SocialError::Unauthorized(_))));

    send_dm_message(
        &setup.state,
        &alice_id,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Новое сообщение владельца".into(),
            image_id: None,
        },
    )
    .await
    .expect("owner can write first");
    send_dm_message(
        &setup.state,
        &bob_id,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Теперь можно ответить".into(),
            image_id: None,
        },
    )
    .await
    .expect("recipient's new message grants a reply");

    update_privacy_settings(&setup.state, &alice_id, UserPrivacySettings::default())
        .await
        .unwrap();
    update_privacy_settings(
        &setup.state,
        &alice_id,
        UserPrivacySettings {
            direct_messages: DirectMessageAudience::Friends,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let denied_again = send_dm_message(
        &setup.state,
        &bob_id,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "После повторного ограничения".into(),
            image_id: None,
        },
    )
    .await;
    assert!(matches!(denied_again, Err(SocialError::Unauthorized(_))));
    let history = list_dm_messages(&setup.state, &bob_id, setup.conversation_id, None)
        .await
        .expect("history remains readable");
    assert_eq!(history.messages.len(), 3);
}

#[tokio::test]
async fn blocking_rejects_both_directions_and_preserves_history_without_restoring_friendship() {
    let setup = setup_pair().await;
    let alice_id = user_id(&setup.alice_user_id);
    let bob_id = user_id(&setup.bob_user_id);
    let original = send_dm_message(
        &setup.state,
        &alice_id,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "История".into(),
            image_id: None,
        },
    )
    .await
    .unwrap()
    .message;

    block_user(&setup.state, &alice_id, &bob_id).await.unwrap();
    let blocks = list_blocked_users(&setup.state, &alice_id).await.unwrap();
    assert_eq!(blocks.users.len(), 1);
    assert_eq!(blocks.users[0].user_id, setup.bob_user_id);
    assert!(
        list_friends(&setup.state, &setup.alice_access_token, Default::default())
            .await
            .unwrap()
            .friends
            .is_empty()
    );
    let history = list_dm_messages(&setup.state, &bob_id, setup.conversation_id.clone(), None)
        .await
        .expect("blocked user retains old history");
    assert_eq!(history.messages.len(), 1);

    let blocked_sender = send_dm_message(
        &setup.state,
        &bob_id,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Заблокирован".into(),
            image_id: None,
        },
    )
    .await;
    assert!(
        matches!(blocked_sender, Err(SocialError::Unauthorized(message)) if message.contains("заблокировал"))
    );
    let blocker_sender = send_dm_message(
        &setup.state,
        &alice_id,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Я блокирую".into(),
            image_id: None,
        },
    )
    .await;
    assert!(matches!(blocker_sender, Err(SocialError::Conflict(_))));
    let edit = edit_dm_message(
        &setup.state,
        &alice_id,
        setup.conversation_id.clone(),
        original.id,
        EditDmMessageRequest {
            body: "Обход через правку".into(),
        },
    )
    .await;
    assert!(matches!(edit, Err(SocialError::Conflict(_))));
    let upload = upload_dm_image(
        &setup.state,
        &alice_id,
        setup.conversation_id.clone(),
        Bytes::from(png_bytes()),
    )
    .await;
    assert!(matches!(upload, Err(SocialError::Conflict(_))));
    let request = send_friend_request(
        &setup.state,
        &setup.bob_access_token,
        SendFriendRequestRequest {
            recipient_user_id: setup.alice_user_id.clone(),
        },
    )
    .await;
    assert!(matches!(request, Err(SocialError::Unauthorized(_))));

    unblock_user(&setup.state, &alice_id, &bob_id)
        .await
        .unwrap();
    assert!(
        list_blocked_users(&setup.state, &alice_id)
            .await
            .unwrap()
            .users
            .is_empty()
    );
    assert!(
        list_friends(&setup.state, &setup.alice_access_token, Default::default())
            .await
            .unwrap()
            .friends
            .is_empty()
    );
    send_dm_message(
        &setup.state,
        &bob_id,
        setup.conversation_id,
        SendDmMessageRequest {
            body: "После разблокировки".into(),
            image_id: None,
        },
    )
    .await
    .expect("unblocked strangers can write under default settings");
}

#[tokio::test]
async fn incoming_contact_audiences_are_independent_and_blocking_takes_precedence() {
    let setup = setup_pair().await;
    let alice_id = user_id(&setup.alice_user_id);
    let bob_id = user_id(&setup.bob_user_id);
    update_privacy_settings(
        &setup.state,
        &alice_id,
        UserPrivacySettings {
            direct_messages: DirectMessageAudience::Everyone,
            direct_calls: IncomingContactAudience::Nobody,
            voice_room_invites: IncomingContactAudience::Everyone,
        },
    )
    .await
    .unwrap();

    assert!(matches!(
        ensure_incoming_call_allowed(&setup.state, &bob_id, &alice_id).await,
        Err(SocialError::Unauthorized(_))
    ));
    ensure_incoming_room_invite_allowed(&setup.state, &bob_id, &alice_id)
        .await
        .expect("invites allowed independently from calls");

    block_user(&setup.state, &alice_id, &bob_id).await.unwrap();
    assert!(matches!(
        ensure_incoming_room_invite_allowed(&setup.state, &bob_id, &alice_id).await,
        Err(SocialError::Unauthorized(message)) if message.contains("заблокировал")
    ));
}

#[tokio::test]
async fn blocking_terminates_pending_calls_invites_and_direct_voice_presence() {
    use crate::features::voice_chat::infrastructure::{
        DirectCall, VoicePresence, VoicePresenceTargetKind, VoiceRoomInvite,
    };
    use cheenhub_contracts::realtime::{VoiceDeviceKind, VoiceRoomInviteStatus};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    let setup = setup_pair().await;
    let alice_id = user_id(&setup.alice_user_id);
    let bob_id = user_id(&setup.bob_user_id);
    let conversation_id = user_id(&setup.conversation_id);
    let now = Utc::now();
    setup
        .state
        .direct_call_store
        .start(DirectCall {
            id: Uuid::new_v4(),
            conversation_id,
            caller_user_id: alice_id,
            callee_user_id: bob_id,
            caller_nickname: "alice_dm".into(),
            caller_avatar_url: None,
            callee_nickname: "bob_dm".into(),
            callee_avatar_url: None,
            started_at: now,
            expires_at: now + Duration::seconds(45),
            answered_at: None,
            callee_notified: false,
        })
        .await
        .expect("call invitation should start");
    setup
        .state
        .voice_room_invite_store
        .start(
            VoiceRoomInvite {
                id: Uuid::new_v4(),
                server_id: Uuid::new_v4(),
                server_name: "Тестовый сервер".into(),
                room_id: Uuid::new_v4(),
                room_name: "Голосовая комната".into(),
                caller_user_id: bob_id,
                caller_nickname: "bob_dm".into(),
                caller_avatar_url: None,
                recipient_user_id: alice_id,
                recipient_nickname: "alice_dm".into(),
                recipient_avatar_url: None,
                status: VoiceRoomInviteStatus::Pending,
                created_at: now,
                expires_at: now + Duration::seconds(45),
                resolved_at: None,
            },
            now,
        )
        .await
        .expect("voice invite should start");
    setup
        .state
        .voice_presence_store
        .join(VoicePresence {
            microphone_enabled: true,
            output_muted: false,
            realtime_stream_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            auth_session_id: Uuid::new_v4(),
            device_kind: VoiceDeviceKind::Desktop,
            voice_protocol_version: 5,
            target_kind: VoicePresenceTargetKind::DirectMessage,
            server_id: conversation_id,
            room_id: conversation_id,
            user_id: alice_id,
            nickname: "alice_dm".into(),
            avatar_url: None,
            joined_at: now,
        })
        .await;

    block_user(&setup.state, &alice_id, &bob_id).await.unwrap();

    assert!(
        setup
            .state
            .direct_call_store
            .list_for_user(&alice_id)
            .await
            .is_empty()
    );
    assert_eq!(
        setup
            .state
            .voice_presence_store
            .active_voice_connection_count()
            .await,
        0
    );
    let invites = setup
        .state
        .voice_room_invite_store
        .list_for_user(&bob_id)
        .await;
    assert_eq!(invites.len(), 1);
    assert_eq!(invites[0].status, VoiceRoomInviteStatus::Cancelled);
}

#[tokio::test]
async fn unblock_revokes_old_permission_to_reply_under_friends_only_policy() {
    let setup = setup_pair().await;
    let alice_id = user_id(&setup.alice_user_id);
    let bob_id = user_id(&setup.bob_user_id);
    super::super::delete_friend(
        &setup.state,
        &setup.alice_access_token,
        setup.bob_user_id.clone(),
    )
    .await
    .unwrap();
    update_privacy_settings(
        &setup.state,
        &alice_id,
        UserPrivacySettings {
            direct_messages: DirectMessageAudience::Friends,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    send_dm_message(
        &setup.state,
        &alice_id,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Разрешаю ответ".into(),
            image_id: None,
        },
    )
    .await
    .unwrap();
    send_dm_message(
        &setup.state,
        &bob_id,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Первый ответ".into(),
            image_id: None,
        },
    )
    .await
    .unwrap();

    block_user(&setup.state, &alice_id, &bob_id).await.unwrap();
    unblock_user(&setup.state, &alice_id, &bob_id)
        .await
        .unwrap();

    let denied = send_dm_message(
        &setup.state,
        &bob_id,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "После разблокировки".into(),
            image_id: None,
        },
    )
    .await;
    assert!(matches!(denied, Err(SocialError::Unauthorized(_))));
    send_dm_message(
        &setup.state,
        &alice_id,
        setup.conversation_id.clone(),
        SendDmMessageRequest {
            body: "Новое разрешение".into(),
            image_id: None,
        },
    )
    .await
    .unwrap();
    send_dm_message(
        &setup.state,
        &bob_id,
        setup.conversation_id,
        SendDmMessageRequest {
            body: "Новый ответ".into(),
            image_id: None,
        },
    )
    .await
    .expect("new message after unblock grants reply");
}

#[tokio::test]
async fn queued_message_is_rejected_after_block_commits_before_contact_guard_releases() {
    let setup = setup_pair().await;
    let alice_id = user_id(&setup.alice_user_id);
    let bob_id = user_id(&setup.bob_user_id);
    let guard = setup
        .state
        .contact_pair_guards
        .lock(&alice_id, &bob_id)
        .await;
    let sending_state = setup.state.clone();
    let conversation_id = setup.conversation_id.clone();
    let pending = tokio::spawn(async move {
        send_dm_message(
            &sending_state,
            &bob_id,
            conversation_id,
            SendDmMessageRequest {
                body: "Запоздалое сообщение".into(),
                image_id: None,
            },
        )
        .await
    });
    tokio::task::yield_now().await;
    setup
        .state
        .social_store
        .block_user(&alice_id, &bob_id, chrono::Utc::now())
        .await
        .unwrap();
    drop(guard);

    let result = pending.await.unwrap();
    assert!(matches!(result, Err(SocialError::Unauthorized(_))));
    let history = list_dm_messages(&setup.state, &alice_id, setup.conversation_id, None)
        .await
        .unwrap();
    assert!(history.messages.is_empty());
}
