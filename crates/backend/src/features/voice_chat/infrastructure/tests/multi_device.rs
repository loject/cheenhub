//! Регрессии атомарного подключения нескольких голосовых устройств одного аккаунта.

use cheenhub_contracts::realtime::VoiceDeviceKind;
use chrono::Utc;
use uuid::Uuid;

use super::super::{
    InMemoryVoicePresenceStore, VoiceJoinConflict, VoicePresence, VoicePresenceTargetKind,
};

fn presence(
    user_id: Uuid,
    auth_session_id: Uuid,
    session_id: Uuid,
    room_id: Uuid,
    target_kind: VoicePresenceTargetKind,
) -> VoicePresence {
    VoicePresence {
        microphone_enabled: true,
        output_muted: false,
        realtime_stream_id: Uuid::new_v4(),
        session_id,
        auth_session_id,
        device_kind: VoiceDeviceKind::Desktop,
        voice_protocol_version: 4,
        target_kind,
        server_id: Uuid::from_u128(100),
        room_id,
        user_id,
        nickname: "участник".to_owned(),
        avatar_url: None,
        joined_at: Utc::now(),
    }
}

#[tokio::test]
async fn media_route_partitions_legacy_and_v6_recipients_by_authenticated_session() {
    let store = InMemoryVoicePresenceStore::default();
    let user = Uuid::new_v4();
    let room = Uuid::new_v4();
    let legacy = VoicePresence {
        voice_protocol_version: 5,
        ..presence(
            user,
            Uuid::new_v4(),
            Uuid::new_v4(),
            room,
            VoicePresenceTargetKind::Server,
        )
    };
    let modern = VoicePresence {
        voice_protocol_version: 6,
        ..presence(
            user,
            Uuid::new_v4(),
            Uuid::new_v4(),
            room,
            VoicePresenceTargetKind::Server,
        )
    };
    store.try_join(legacy.clone(), false).await.unwrap();
    store.try_join(modern.clone(), false).await.unwrap();

    let route = store
        .media_route_for_session(
            VoicePresenceTargetKind::Server,
            &room,
            &modern.session_id,
            &user,
        )
        .unwrap();
    assert_eq!(route.recipients.len(), 2);
    assert_eq!(route.legacy_recipients.as_ref(), &[legacy.session_id]);
    assert_eq!(route.modern_recipients.as_ref(), &[modern.session_id]);
}

#[tokio::test]
async fn two_auth_devices_can_share_one_server_room_and_keep_distinct_media_routes() {
    let store = InMemoryVoicePresenceStore::default();
    let user = Uuid::new_v4();
    let room = Uuid::new_v4();
    let first = presence(
        user,
        Uuid::new_v4(),
        Uuid::new_v4(),
        room,
        VoicePresenceTargetKind::Server,
    );
    let second = presence(
        user,
        Uuid::new_v4(),
        Uuid::new_v4(),
        room,
        VoicePresenceTargetKind::Server,
    );

    assert!(
        store
            .try_join(first.clone(), false)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .try_join(second.clone(), false)
            .await
            .unwrap()
            .is_empty()
    );
    let participants = store
        .room_participants(VoicePresenceTargetKind::Server, &first.server_id, &room)
        .await;
    assert_eq!(participants.len(), 2);
    assert_eq!(store.active_voice_connection_count().await, 2);
    for member in [&first, &second] {
        let route = store
            .media_route_for_session(
                VoicePresenceTargetKind::Server,
                &room,
                &member.session_id,
                &user,
            )
            .unwrap();
        assert_eq!(route.presence.session_id, member.session_id);
        assert_eq!(route.recipients.len(), 2);
        assert!(route.recipients.contains(&first.session_id));
        assert!(route.recipients.contains(&second.session_id));
    }
    store.leave_realtime_stream(&first.realtime_stream_id).await;
    assert!(
        store
            .media_route_for_session(
                VoicePresenceTargetKind::Server,
                &room,
                &first.session_id,
                &user
            )
            .is_none()
    );
    assert!(
        store
            .media_route_for_session(
                VoicePresenceTargetKind::Server,
                &room,
                &second.session_id,
                &user
            )
            .is_some()
    );
}

#[tokio::test]
async fn moving_another_device_to_a_different_room_requires_confirmation_without_mutation() {
    let store = InMemoryVoicePresenceStore::default();
    let user = Uuid::new_v4();
    let first = presence(
        user,
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        VoicePresenceTargetKind::Server,
    );
    let second = presence(
        user,
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        VoicePresenceTargetKind::Server,
    );
    store.try_join(first.clone(), false).await.unwrap();

    assert!(matches!(
        store.try_join(second.clone(), false).await,
        Err(VoiceJoinConflict::DeviceTransitionConfirmationRequired)
    ));
    assert!(
        store
            .presence_for_stream(&first.realtime_stream_id, &user)
            .is_some()
    );
    assert!(
        store
            .presence_for_stream(&second.realtime_stream_id, &user)
            .is_none()
    );
    let removed = store.try_join(second.clone(), true).await.unwrap();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].session_id, first.session_id);
    assert!(
        store
            .presence_for_stream(&first.realtime_stream_id, &user)
            .is_none()
    );
    assert!(
        store
            .presence_for_stream(&second.realtime_stream_id, &user)
            .is_some()
    );
}

#[tokio::test]
async fn second_tab_with_one_auth_session_requires_distinct_confirmation() {
    let store = InMemoryVoicePresenceStore::default();
    let user = Uuid::new_v4();
    let auth = Uuid::new_v4();
    let room = Uuid::new_v4();
    let first = VoicePresence {
        voice_protocol_version: 5,
        ..presence(
            user,
            auth,
            Uuid::new_v4(),
            room,
            VoicePresenceTargetKind::Server,
        )
    };
    let second = VoicePresence {
        voice_protocol_version: 5,
        ..presence(
            user,
            auth,
            Uuid::new_v4(),
            room,
            VoicePresenceTargetKind::Server,
        )
    };
    store.try_join(first.clone(), false).await.unwrap();

    assert!(matches!(
        store.try_join(second.clone(), false).await,
        Err(VoiceJoinConflict::SameAuthSessionConfirmationRequired)
    ));
    assert_eq!(store.active_voice_connection_count().await, 1);
    assert!(
        store
            .presence_for_stream(&first.realtime_stream_id, &user)
            .is_some()
    );
    let removed = store.try_join(second.clone(), true).await.unwrap();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].session_id, first.session_id);
    assert_eq!(store.active_voice_connection_count().await, 1);
}

#[tokio::test]
async fn legacy_reconnection_to_same_room_replaces_previous_tab_without_confirmation() {
    let store = InMemoryVoicePresenceStore::default();
    let user = Uuid::new_v4();
    let auth = Uuid::new_v4();
    let room = Uuid::new_v4();
    let first = presence(
        user,
        auth,
        Uuid::new_v4(),
        room,
        VoicePresenceTargetKind::Server,
    );
    let second = presence(
        user,
        auth,
        Uuid::new_v4(),
        room,
        VoicePresenceTargetKind::Server,
    );
    store.try_join(first.clone(), false).await.unwrap();

    let removed = store
        .try_join(second.clone(), false)
        .await
        .expect("legacy reconnect to the same room must not require an unavailable dialog");

    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].session_id, first.session_id);
    let remaining = store
        .room_participants(VoicePresenceTargetKind::Server, &first.server_id, &room)
        .await;
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].session_id, second.session_id);
}

#[tokio::test]
async fn legacy_direct_call_reconnect_replaces_same_auth_session_without_confirmation() {
    let store = InMemoryVoicePresenceStore::default();
    let user = Uuid::new_v4();
    let auth = Uuid::new_v4();
    let room = Uuid::new_v4();
    let first = presence(
        user,
        auth,
        Uuid::new_v4(),
        room,
        VoicePresenceTargetKind::DirectMessage,
    );
    let second = presence(
        user,
        auth,
        Uuid::new_v4(),
        room,
        VoicePresenceTargetKind::DirectMessage,
    );
    store.try_join(first.clone(), false).await.unwrap();

    let removed = store
        .try_join(second.clone(), false)
        .await
        .expect("legacy DM reconnect must not require an unavailable confirmation dialog");

    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].session_id, first.session_id);
    assert!(
        store
            .presence_for_stream(&first.realtime_stream_id, &user)
            .is_none()
    );
    assert_eq!(
        store
            .presence_for_stream(&second.realtime_stream_id, &user)
            .unwrap()
            .session_id,
        second.session_id
    );
    assert_eq!(store.active_voice_connection_count().await, 1);
}

#[tokio::test]
async fn legacy_client_cannot_silently_replace_active_v5_tab() {
    let store = InMemoryVoicePresenceStore::default();
    let user = Uuid::new_v4();
    let auth = Uuid::new_v4();
    let room = Uuid::new_v4();
    let current = VoicePresence {
        voice_protocol_version: 5,
        ..presence(
            user,
            auth,
            Uuid::new_v4(),
            room,
            VoicePresenceTargetKind::Server,
        )
    };
    let legacy = presence(
        user,
        auth,
        Uuid::new_v4(),
        room,
        VoicePresenceTargetKind::Server,
    );
    store.try_join(current.clone(), false).await.unwrap();

    assert!(matches!(
        store.try_join(legacy, false).await,
        Err(VoiceJoinConflict::SameAuthSessionConfirmationRequired)
    ));
    assert!(
        store
            .presence_for_stream(&current.realtime_stream_id, &user)
            .is_some()
    );
}

#[tokio::test]
async fn personal_voice_call_remains_single_device_even_with_distinct_auth_sessions() {
    let store = InMemoryVoicePresenceStore::default();
    let user = Uuid::new_v4();
    let room = Uuid::new_v4();
    let first = presence(
        user,
        Uuid::new_v4(),
        Uuid::new_v4(),
        room,
        VoicePresenceTargetKind::DirectMessage,
    );
    let second = presence(
        user,
        Uuid::new_v4(),
        Uuid::new_v4(),
        room,
        VoicePresenceTargetKind::DirectMessage,
    );
    store.try_join(first.clone(), false).await.unwrap();

    assert!(matches!(
        store.try_join(second.clone(), false).await,
        Err(VoiceJoinConflict::DeviceTransitionConfirmationRequired)
    ));
    assert!(store.try_join(second.clone(), true).await.is_ok());
    let participants = store
        .room_participants(
            VoicePresenceTargetKind::DirectMessage,
            &first.server_id,
            &room,
        )
        .await;
    assert_eq!(participants.len(), 1);
    assert_eq!(participants[0].session_id, second.session_id);
}
