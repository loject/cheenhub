//! Личные сообщения без дружбы с сохранением границ доступа.

use super::*;
use crate::features::social::SocialError;
use crate::realtime::{EnvelopeSink, WebSocketOutbound};
use cheenhub_contracts::realtime::{RealtimeKind, RealtimeModule, SocialKind};
use cheenhub_contracts::rest::UserRelationStatus;

fn social_envelope(event: WebSocketOutbound) -> cheenhub_contracts::realtime::RealtimeEnvelope {
    match event {
        WebSocketOutbound::Envelope(envelope) => envelope,
        WebSocketOutbound::PreparedEnvelope(text) => serde_json::from_str(&text).unwrap(),
        _ => panic!("expected social envelope"),
    }
}

#[tokio::test]
async fn non_friends_exchange_messages_and_recipient_gets_unread_dialog() {
    let state = state();
    let alice = registered_user(&state, "alice_stranger", "alice-stranger@example.com").await;
    let bob = registered_user(&state, "bob_stranger", "bob-stranger@example.com").await;
    let (sender, mut events) = tokio::sync::mpsc::channel(16);
    state
        .realtime_hub
        .register_stream(
            uuid::Uuid::new_v4(),
            RealtimeModule::Social,
            user_id(&bob.user.id),
            EnvelopeSink::websocket(sender),
        )
        .await;

    let opened = open_dm_conversation(
        &state,
        &user_id(&alice.user.id),
        OpenDmConversationRequest {
            friend_user_id: bob.user.id.clone(),
        },
    )
    .await
    .expect("non-friend dialog should open")
    .conversation;
    assert_eq!(opened.relation, None);
    while let Ok(event) = events.try_recv() {
        let event = social_envelope(event);
        assert_ne!(
            event.kind,
            RealtimeKind::Social(SocialKind::DirectMessageCreated)
        );
    }
    let before = list_dm_conversations(&state, &user_id(&bob.user.id))
        .await
        .unwrap();
    assert_eq!(before.conversations[0].unread_count, 0);

    send_dm_message(
        &state,
        &user_id(&alice.user.id),
        opened.id.clone(),
        SendDmMessageRequest {
            body: "Привет из голосовой комнаты".to_owned(),
            image_id: None,
        },
    )
    .await
    .expect("non-friend message should send");
    let mut notifications = 0;
    while let Ok(event) = events.try_recv() {
        let event = social_envelope(event);
        if event.kind == RealtimeKind::Social(SocialKind::DirectMessageCreated) {
            assert_eq!(event.payload["conversation_id"], opened.id);
            assert_eq!(event.payload["sender_user_id"], alice.user.id);
            notifications += 1;
        }
    }
    assert_eq!(notifications, 1);

    let received = list_dm_conversations(&state, &user_id(&bob.user.id))
        .await
        .unwrap();
    assert_eq!(received.conversations[0].id, opened.id);
    assert_eq!(received.conversations[0].unread_count, 1);
    let history = list_dm_messages(&state, &user_id(&bob.user.id), opened.id.clone(), None)
        .await
        .unwrap();
    assert_eq!(history.messages[0].body, "Привет из голосовой комнаты");
    send_dm_message(
        &state,
        &user_id(&bob.user.id),
        opened.id.clone(),
        SendDmMessageRequest {
            body: "Привет".to_owned(),
            image_id: None,
        },
    )
    .await
    .expect("recipient can reply without friendship");

    let voice = super::super::direct_message_voice_access(
        &state,
        &user_id(&bob.user.id),
        &user_id(&opened.id),
    )
    .await;
    assert!(matches!(voice, Err(SocialError::Unauthorized(_))));
    let charlie = registered_user(&state, "charlie_stranger", "charlie-stranger@example.com").await;
    let denied = send_dm_message(
        &state,
        &user_id(&charlie.user.id),
        opened.id,
        SendDmMessageRequest {
            body: "Чужой диалог".to_owned(),
            image_id: None,
        },
    )
    .await;
    assert!(matches!(denied, Err(SocialError::NotFound(_))));
}

#[tokio::test]
async fn dialog_relation_tracks_friend_request_acceptance_and_removal() {
    let state = state();
    let alice = registered_user(&state, "alice_relation", "alice-relation@example.com").await;
    let bob = registered_user(&state, "bob_relation", "bob-relation@example.com").await;
    let opened = open_dm_conversation(
        &state,
        &user_id(&alice.user.id),
        OpenDmConversationRequest {
            friend_user_id: bob.user.id.clone(),
        },
    )
    .await
    .unwrap()
    .conversation;
    assert_eq!(opened.relation, None);

    let request = send_friend_request(
        &state,
        &alice.access_token,
        SendFriendRequestRequest {
            recipient_user_id: bob.user.id.clone(),
        },
    )
    .await
    .unwrap();

    let alice_view = list_dm_conversations(&state, &user_id(&alice.user.id))
        .await
        .unwrap();
    let bob_view = list_dm_conversations(&state, &user_id(&bob.user.id))
        .await
        .unwrap();
    assert_eq!(
        alice_view.conversations[0].relation,
        Some(UserRelationStatus::PendingOutgoing)
    );
    assert_eq!(
        bob_view.conversations[0].relation,
        Some(UserRelationStatus::PendingIncoming)
    );

    accept_friend_request(&state, &bob.access_token, request.request.id)
        .await
        .unwrap();

    let friends = list_dm_conversations(&state, &user_id(&bob.user.id))
        .await
        .unwrap();
    assert_eq!(
        friends.conversations[0].relation,
        Some(UserRelationStatus::Friends)
    );
    assert_eq!(friends.conversations[0].id, opened.id);

    super::super::delete_friend(&state, &alice.access_token, bob.user.id.clone())
        .await
        .unwrap();

    let removed = list_dm_conversations(&state, &user_id(&bob.user.id))
        .await
        .unwrap();
    assert_eq!(removed.conversations[0].relation, None);
    send_dm_message(
        &state,
        &user_id(&bob.user.id),
        opened.id,
        SendDmMessageRequest {
            body: "Переписка продолжается".to_owned(),
            image_id: None,
        },
    )
    .await
    .expect("removing friend must not block messages");
}

#[tokio::test]
async fn opening_direct_dialog_with_self_is_rejected() {
    let state = state();
    let alice = registered_user(&state, "alice_self", "alice-self@example.com").await;

    let result = open_dm_conversation(
        &state,
        &user_id(&alice.user.id),
        OpenDmConversationRequest {
            friend_user_id: alice.user.id,
        },
    )
    .await;

    assert!(matches!(result, Err(SocialError::BadRequest(_))));
}
