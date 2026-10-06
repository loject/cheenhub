//! Регрессионные проверки индексов маршрутизации голосовых комнат.

use chrono::Utc;
use uuid::Uuid;

use super::super::{
    InMemoryVoicePresenceStore, VoicePresence, VoicePresenceTarget, VoicePresenceTargetKind,
};

fn presence(
    realtime_stream_id: Uuid,
    session_id: Uuid,
    server_id: Uuid,
    room_id: Uuid,
    user_id: Uuid,
) -> VoicePresence {
    VoicePresence {
        realtime_stream_id,
        session_id,
        target_kind: VoicePresenceTargetKind::Server,
        server_id,
        room_id,
        user_id,
        nickname: "voice_user".to_owned(),
        avatar_url: None,
        joined_at: Utc::now(),
    }
}

fn direct_message_presence(
    realtime_stream_id: Uuid,
    session_id: Uuid,
    room_id: Uuid,
    user_id: Uuid,
) -> VoicePresence {
    VoicePresence {
        realtime_stream_id,
        session_id,
        target_kind: VoicePresenceTargetKind::DirectMessage,
        server_id: Uuid::new_v4(),
        room_id,
        user_id,
        nickname: "voice_user".to_owned(),
        avatar_url: None,
        joined_at: Utc::now(),
    }
}

#[tokio::test]
async fn server_and_direct_message_routes_are_isolated_for_the_same_room_id() {
    let store = InMemoryVoicePresenceStore::default();
    let room_id = Uuid::new_v4();
    let server_user_id = Uuid::new_v4();
    let server_session_id = Uuid::new_v4();
    let direct_message_user_id = Uuid::new_v4();
    let direct_message_session_id = Uuid::new_v4();
    let server_id = Uuid::new_v4();

    store
        .join(presence(
            Uuid::new_v4(),
            server_session_id,
            server_id,
            room_id,
            server_user_id,
        ))
        .await;
    let direct_message_presence = direct_message_presence(
        Uuid::new_v4(),
        direct_message_session_id,
        room_id,
        direct_message_user_id,
    );
    store.join(direct_message_presence).await;

    let server_route = store
        .media_route(VoicePresenceTargetKind::Server, &room_id, &server_user_id)
        .expect("server participant should have a route");
    let direct_message_route = store
        .media_route(
            VoicePresenceTargetKind::DirectMessage,
            &room_id,
            &direct_message_user_id,
        )
        .expect("direct message participant should have a route");

    assert_eq!(server_route.recipients.as_ref(), &[server_session_id]);
    assert_eq!(
        direct_message_route.recipients.as_ref(),
        &[direct_message_session_id]
    );
}

#[tokio::test]
async fn media_route_reuses_unaffected_room_snapshot_after_another_room_changes() {
    let store = InMemoryVoicePresenceStore::default();
    let server_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    store
        .join(presence(
            Uuid::new_v4(),
            Uuid::new_v4(),
            server_id,
            room_id,
            user_id,
        ))
        .await;
    let before = store
        .media_route(VoicePresenceTargetKind::Server, &room_id, &user_id)
        .expect("joined room should have a media route");
    store
        .join(presence(
            Uuid::new_v4(),
            Uuid::new_v4(),
            server_id,
            Uuid::new_v4(),
            Uuid::new_v4(),
        ))
        .await;
    let after = store
        .media_route(VoicePresenceTargetKind::Server, &room_id, &user_id)
        .expect("unmodified room should keep its media route");

    assert!(std::sync::Arc::ptr_eq(&before.presence, &after.presence));
    assert!(std::sync::Arc::ptr_eq(
        &before.recipients,
        &after.recipients
    ));
}

#[tokio::test]
async fn joining_another_room_removes_old_route_before_finishing() {
    let store = InMemoryVoicePresenceStore::default();
    let server_id = Uuid::new_v4();
    let old_room_id = Uuid::new_v4();
    let new_room_id = Uuid::new_v4();
    let stream_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let old_session_id = Uuid::new_v4();
    let new_session_id = Uuid::new_v4();

    store
        .join(presence(
            stream_id,
            old_session_id,
            server_id,
            old_room_id,
            user_id,
        ))
        .await;
    store
        .join(presence(
            stream_id,
            new_session_id,
            server_id,
            new_room_id,
            user_id,
        ))
        .await;

    assert!(
        store
            .media_route(VoicePresenceTargetKind::Server, &old_room_id, &user_id)
            .is_none()
    );
    let new_route = store
        .media_route(VoicePresenceTargetKind::Server, &new_room_id, &user_id)
        .expect("new room route should be published");
    assert_eq!(new_route.presence.session_id, new_session_id);
    assert_eq!(new_route.recipients.as_ref(), &[new_session_id]);
    assert_eq!(
        store
            .presence_for_stream(&stream_id, &user_id)
            .expect("stream lookup should reflect the completed transition")
            .session_id,
        new_session_id
    );
}

#[tokio::test]
async fn replacing_presence_in_the_same_room_keeps_only_the_new_session() {
    let store = InMemoryVoicePresenceStore::default();
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let old_stream_id = Uuid::new_v4();
    let new_stream_id = Uuid::new_v4();
    let old_session_id = Uuid::new_v4();
    let new_session_id = Uuid::new_v4();
    let server_id = Uuid::new_v4();

    store
        .join(presence(
            old_stream_id,
            old_session_id,
            server_id,
            room_id,
            user_id,
        ))
        .await;
    store
        .join(presence(
            new_stream_id,
            new_session_id,
            server_id,
            room_id,
            user_id,
        ))
        .await;

    let route = store
        .media_route(VoicePresenceTargetKind::Server, &room_id, &user_id)
        .expect("replacement should keep a route in the same room");
    assert_eq!(route.presence.session_id, new_session_id);
    assert_eq!(route.recipients.as_ref(), &[new_session_id]);
    assert!(
        store
            .presence_for_stream(&old_stream_id, &user_id)
            .is_none()
    );
    assert_eq!(
        store
            .presence_for_stream(&new_stream_id, &user_id)
            .expect("new stream should be indexed")
            .session_id,
        new_session_id
    );
}

#[test]
fn join_publication_plan_places_addition_after_all_removals() {
    let removed_room = VoicePresenceTarget {
        kind: VoicePresenceTargetKind::Server,
        server_id: Uuid::new_v4(),
        room_id: Uuid::new_v4(),
    };
    let other_removed_room = VoicePresenceTarget {
        kind: VoicePresenceTargetKind::DirectMessage,
        server_id: Uuid::new_v4(),
        room_id: Uuid::new_v4(),
    };
    let added_room = VoicePresenceTarget {
        kind: VoicePresenceTargetKind::Server,
        server_id: Uuid::new_v4(),
        room_id: Uuid::new_v4(),
    };
    let old_rooms = [removed_room, other_removed_room].into_iter().collect();

    let plan = super::super::presence_registry::join_publication_plan(old_rooms, added_room);

    assert_eq!(
        plan.last(),
        Some(&super::super::presence_registry::RoomPublication::Addition(
            added_room
        ))
    );
    assert!(plan[..plan.len() - 1].iter().all(|publication| matches!(
        publication,
        super::super::presence_registry::RoomPublication::Removal(_)
    )));
    assert!(
        plan.contains(&super::super::presence_registry::RoomPublication::Removal(
            removed_room
        ))
    );
    assert!(
        plan.contains(&super::super::presence_registry::RoomPublication::Removal(
            other_removed_room
        ))
    );

    let same_room_plan = super::super::presence_registry::join_publication_plan(
        [added_room].into_iter().collect(),
        added_room,
    );
    assert_eq!(
        same_room_plan,
        vec![super::super::presence_registry::RoomPublication::Addition(
            added_room
        )]
    );
}
