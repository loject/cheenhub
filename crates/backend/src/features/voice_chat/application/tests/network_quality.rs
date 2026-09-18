//! Тесты публикации сетевых метрик голосового общения.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use cheenhub_contracts::realtime::{
    JoinVoiceRoom, PublishVoiceNetworkQuality, RealtimeModule, VoiceNetworkTargetKind,
};
use cheenhub_contracts::rest::ServerRoomKind;
use chrono::Utc;
use uuid::Uuid;

use super::{create_room, registered_user, state};
use crate::features::voice_chat::application::{
    VoiceChatApplicationError, authorize_network_quality_publication_at, join_room,
    prepare_network_quality_broadcast, publish_network_quality,
};
use crate::features::voice_chat::infrastructure::{VoicePresence, VoicePresenceTargetKind};
use crate::realtime::{EnvelopeSink, WebSocketOutbound};

async fn insert_presence(
    state: &crate::state::AppState,
    realtime_stream_id: Uuid,
    user_id: Uuid,
    target_kind: VoicePresenceTargetKind,
    server_id: Uuid,
    room_id: Uuid,
) {
    state
        .voice_presence_store
        .join(VoicePresence {
            realtime_stream_id,
            session_id: Uuid::new_v4(),
            target_kind,
            server_id,
            room_id,
            user_id,
            nickname: format!("voice-{user_id}"),
            avatar_url: None,
            joined_at: Utc::now(),
        })
        .await;
}

#[tokio::test]
async fn network_quality_requires_active_voice_presence() {
    let state = state();
    let (_user, user_id) = registered_user(&state).await;

    let error = authorize_network_quality_publication_at(
        &state,
        uuid::Uuid::new_v4(),
        &user_id,
        PublishVoiceNetworkQuality { rtt_ms: 42 },
        Instant::now(),
    )
    .await
    .expect_err("network quality outside voice presence should fail");

    assert!(matches!(error, VoiceChatApplicationError::Unauthorized(_)));
}

#[tokio::test]
async fn network_quality_accepts_maximum_rtt_and_rejects_value_above_it() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = uuid::Uuid::new_v4();
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;
    join_room(
        &state,
        stream_id,
        uuid::Uuid::new_v4(),
        &user,
        &user_id,
        JoinVoiceRoom { server_id, room_id },
    )
    .await
    .expect("join should succeed");

    let accepted = authorize_network_quality_publication_at(
        &state,
        stream_id,
        &user_id,
        PublishVoiceNetworkQuality { rtt_ms: 5_000 },
        Instant::now(),
    )
    .await
    .expect("maximum RTT should be valid");
    let error = authorize_network_quality_publication_at(
        &state,
        stream_id,
        &user_id,
        PublishVoiceNetworkQuality { rtt_ms: 5_001 },
        Instant::now(),
    )
    .await
    .expect_err("impossible RTT should fail");

    assert!(accepted.is_some());
    assert!(matches!(error, VoiceChatApplicationError::BadRequest(_)));
}

#[tokio::test]
async fn network_quality_is_scoped_to_the_active_room() {
    let state = state();
    let server_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let other_room_id = Uuid::new_v4();
    let alice_id = Uuid::new_v4();
    let bob_id = Uuid::new_v4();
    let carol_id = Uuid::new_v4();
    let alice_stream_id = Uuid::new_v4();
    insert_presence(
        &state,
        alice_stream_id,
        alice_id,
        VoicePresenceTargetKind::Server,
        server_id,
        room_id,
    )
    .await;
    insert_presence(
        &state,
        Uuid::new_v4(),
        bob_id,
        VoicePresenceTargetKind::Server,
        server_id,
        room_id,
    )
    .await;
    insert_presence(
        &state,
        Uuid::new_v4(),
        carol_id,
        VoicePresenceTargetKind::Server,
        server_id,
        other_room_id,
    )
    .await;

    let publication = authorize_network_quality_publication_at(
        &state,
        alice_stream_id,
        &alice_id,
        PublishVoiceNetworkQuality { rtt_ms: 37 },
        Instant::now(),
    )
    .await
    .expect("active participant can publish network quality")
    .expect("first publication should pass rate limit");
    let broadcast = prepare_network_quality_broadcast(&state, publication).await;

    assert_eq!(broadcast.event.target_kind, VoiceNetworkTargetKind::Server);
    assert_eq!(broadcast.event.server_id, server_id.to_string());
    assert_eq!(broadcast.event.room_id, room_id.to_string());
    assert_eq!(broadcast.event.user_id, alice_id.to_string());
    assert_eq!(broadcast.event.rtt_ms, 37);
    assert_eq!(
        broadcast
            .recipient_user_ids
            .into_iter()
            .collect::<HashSet<_>>(),
        HashSet::from([alice_id, bob_id])
    );
}

#[tokio::test]
async fn direct_message_network_quality_is_scoped_to_one_conversation() {
    let state = state();
    let conversation_id = Uuid::new_v4();
    let other_conversation_id = Uuid::new_v4();
    let alice_id = Uuid::new_v4();
    let bob_id = Uuid::new_v4();
    let carol_id = Uuid::new_v4();
    let alice_stream_id = Uuid::new_v4();
    insert_presence(
        &state,
        alice_stream_id,
        alice_id,
        VoicePresenceTargetKind::DirectMessage,
        conversation_id,
        conversation_id,
    )
    .await;
    insert_presence(
        &state,
        Uuid::new_v4(),
        bob_id,
        VoicePresenceTargetKind::DirectMessage,
        conversation_id,
        conversation_id,
    )
    .await;
    insert_presence(
        &state,
        Uuid::new_v4(),
        carol_id,
        VoicePresenceTargetKind::DirectMessage,
        other_conversation_id,
        other_conversation_id,
    )
    .await;

    let publication = authorize_network_quality_publication_at(
        &state,
        alice_stream_id,
        &alice_id,
        PublishVoiceNetworkQuality { rtt_ms: 45 },
        Instant::now(),
    )
    .await
    .expect("active direct call participant can publish network quality")
    .expect("first publication should pass rate limit");
    let broadcast = prepare_network_quality_broadcast(&state, publication).await;

    assert_eq!(
        broadcast.event.target_kind,
        VoiceNetworkTargetKind::DirectMessage
    );
    assert_eq!(broadcast.event.server_id, conversation_id.to_string());
    assert_eq!(broadcast.event.room_id, conversation_id.to_string());
    assert_eq!(
        broadcast
            .recipient_user_ids
            .into_iter()
            .collect::<HashSet<_>>(),
        HashSet::from([alice_id, bob_id])
    );
}

#[tokio::test]
async fn rate_limit_is_decided_before_recipient_collection() {
    let state = state();
    let server_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let stream_id = Uuid::new_v4();
    insert_presence(
        &state,
        stream_id,
        user_id,
        VoicePresenceTargetKind::Server,
        server_id,
        room_id,
    )
    .await;
    let started_at = Instant::now();

    let first = authorize_network_quality_publication_at(
        &state,
        stream_id,
        &user_id,
        PublishVoiceNetworkQuality { rtt_ms: 20 },
        started_at,
    )
    .await
    .expect("valid publication should be authorized");
    let rate_limited = authorize_network_quality_publication_at(
        &state,
        stream_id,
        &user_id,
        PublishVoiceNetworkQuality { rtt_ms: 21 },
        started_at + Duration::from_millis(499),
    )
    .await
    .expect("rate limit should not return an application error");

    assert!(first.is_some());
    assert!(rate_limited.is_none());
}

#[tokio::test]
async fn repeated_network_quality_publication_is_dropped_before_fanout() {
    let state = state();
    let server_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let stream_id = Uuid::new_v4();
    insert_presence(
        &state,
        stream_id,
        user_id,
        VoicePresenceTargetKind::Server,
        server_id,
        room_id,
    )
    .await;
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
    state
        .realtime_hub
        .register_stream(
            stream_id,
            RealtimeModule::VoiceChat,
            user_id,
            EnvelopeSink::websocket(sender),
        )
        .await;

    publish_network_quality(
        &state,
        stream_id,
        &user_id,
        PublishVoiceNetworkQuality { rtt_ms: 20 },
    )
    .await
    .expect("first publication should pass");
    publish_network_quality(
        &state,
        stream_id,
        &user_id,
        PublishVoiceNetworkQuality { rtt_ms: 21 },
    )
    .await
    .expect("rate-limited publication should be dropped without an error");

    assert!(matches!(
        receiver.try_recv(),
        Ok(WebSocketOutbound::Envelope(_))
    ));
    assert!(receiver.try_recv().is_err());
    assert_eq!(state.voice_presence_store.room_participants_call_count(), 1);
}

#[tokio::test]
async fn network_quality_fanout_reaches_only_streams_in_the_active_room() {
    let state = state();
    let server_id = Uuid::new_v4();
    let room_a_id = Uuid::new_v4();
    let room_b_id = Uuid::new_v4();
    let alice_id = Uuid::new_v4();
    let bob_id = Uuid::new_v4();
    let carol_id = Uuid::new_v4();
    let alice_stream_id = Uuid::new_v4();
    let bob_stream_id = Uuid::new_v4();
    let carol_stream_id = Uuid::new_v4();
    insert_presence(
        &state,
        alice_stream_id,
        alice_id,
        VoicePresenceTargetKind::Server,
        server_id,
        room_a_id,
    )
    .await;
    insert_presence(
        &state,
        bob_stream_id,
        bob_id,
        VoicePresenceTargetKind::Server,
        server_id,
        room_a_id,
    )
    .await;
    insert_presence(
        &state,
        carol_stream_id,
        carol_id,
        VoicePresenceTargetKind::Server,
        server_id,
        room_b_id,
    )
    .await;

    let (alice_sender, mut alice_receiver) = tokio::sync::mpsc::channel(2);
    let (bob_sender, mut bob_receiver) = tokio::sync::mpsc::channel(2);
    let (carol_sender, mut carol_receiver) = tokio::sync::mpsc::channel(2);
    for (stream_id, user_id, sender) in [
        (alice_stream_id, alice_id, alice_sender),
        (bob_stream_id, bob_id, bob_sender),
        (carol_stream_id, carol_id, carol_sender),
    ] {
        state
            .realtime_hub
            .register_stream(
                stream_id,
                RealtimeModule::VoiceChat,
                user_id,
                EnvelopeSink::websocket(sender),
            )
            .await;
    }

    publish_network_quality(
        &state,
        alice_stream_id,
        &alice_id,
        PublishVoiceNetworkQuality { rtt_ms: 42 },
    )
    .await
    .expect("room A participant should publish network quality");

    assert!(matches!(
        alice_receiver.try_recv(),
        Ok(WebSocketOutbound::Envelope(_))
    ));
    assert!(matches!(
        bob_receiver.try_recv(),
        Ok(WebSocketOutbound::Envelope(_))
    ));
    assert!(carol_receiver.try_recv().is_err());
}
