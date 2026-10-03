//! Проверки хранения голосового присутствия.

use cheenhub_contracts::media::MediaDatagramKind;
use cheenhub_contracts::video_presets::{
    BASE_CAMERA_VIDEO_PRESETS, BASE_SCREEN_SHARE_VIDEO_PRESETS,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

use super::super::super::media_policy::VideoAdmission;
use super::super::super::media_policy::tests::support::key_frame_datagram;
use super::super::uplink::MicrophoneUplinkGrant;
use super::super::{InMemoryVoicePresenceStore, VoicePresence, VoicePresenceTargetKind};

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

#[tokio::test]
async fn room_presence_authorizes_only_joined_users() {
    let store = InMemoryVoicePresenceStore::default();
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    assert!(
        store
            .room_presence_for_user(VoicePresenceTargetKind::Server, &room_id, &user_id)
            .await
            .is_none()
    );

    store
        .join(presence(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            room_id,
            user_id,
        ))
        .await;

    assert!(
        store
            .room_presence_for_user(VoicePresenceTargetKind::Server, &room_id, &user_id)
            .await
            .is_some()
    );
}

#[tokio::test]
async fn media_recipients_exclude_sender_and_other_rooms() {
    let store = InMemoryVoicePresenceStore::default();
    let server_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let other_room_id = Uuid::new_v4();
    let sender_session_id = Uuid::new_v4();
    let recipient_session_id = Uuid::new_v4();
    let other_room_session_id = Uuid::new_v4();

    store
        .join(presence(
            Uuid::new_v4(),
            sender_session_id,
            server_id,
            room_id,
            Uuid::new_v4(),
        ))
        .await;
    store
        .join(presence(
            Uuid::new_v4(),
            recipient_session_id,
            server_id,
            room_id,
            Uuid::new_v4(),
        ))
        .await;
    store
        .join(presence(
            Uuid::new_v4(),
            other_room_session_id,
            server_id,
            other_room_id,
            Uuid::new_v4(),
        ))
        .await;

    let recipients = store
        .media_recipient_sessions(
            VoicePresenceTargetKind::Server,
            &room_id,
            &sender_session_id,
        )
        .await;

    assert_eq!(recipients, vec![recipient_session_id]);
}

#[tokio::test]
async fn replacing_user_presence_makes_old_session_stale() {
    let store = InMemoryVoicePresenceStore::default();
    let server_id = Uuid::new_v4();
    let first_room_id = Uuid::new_v4();
    let second_room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let old_session_id = Uuid::new_v4();
    let new_session_id = Uuid::new_v4();

    store
        .join(presence(
            Uuid::new_v4(),
            old_session_id,
            server_id,
            first_room_id,
            user_id,
        ))
        .await;
    store
        .join(presence(
            Uuid::new_v4(),
            new_session_id,
            server_id,
            second_room_id,
            user_id,
        ))
        .await;

    assert!(
        store
            .room_presence_for_user(VoicePresenceTargetKind::Server, &first_room_id, &user_id)
            .await
            .is_none()
    );
    assert_eq!(
        store
            .room_presence_for_user(VoicePresenceTargetKind::Server, &second_room_id, &user_id)
            .await
            .expect("new presence should remain")
            .session_id,
        new_session_id
    );
}

#[tokio::test]
async fn disconnect_removes_voice_connection_and_video_activity() {
    let store = InMemoryVoicePresenceStore::default();
    let server_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let stream_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    store
        .join(presence(stream_id, session_id, server_id, room_id, user_id))
        .await;
    let camera = key_frame_datagram(MediaDatagramKind::CameraFrame, room_id);
    assert_eq!(
        store
            .inspect_video_datagram(session_id, &camera, BASE_CAMERA_VIDEO_PRESETS)
            .await,
        VideoAdmission::Forward
    );
    let mut screen = key_frame_datagram(MediaDatagramKind::ScreenFrame, room_id);
    screen.sequence = 2;
    assert_eq!(
        store
            .inspect_video_datagram(session_id, &screen, BASE_SCREEN_SHARE_VIDEO_PRESETS)
            .await,
        VideoAdmission::Forward
    );

    assert_eq!(store.active_voice_connection_count().await, 1);
    assert_eq!(store.active_video_source_count().await, 2);

    store.leave_realtime_stream(&stream_id).await;

    assert_eq!(store.active_voice_connection_count().await, 0);
    assert_eq!(
        store.active_video_source_count().await,
        0,
        "отключение realtime-потока убирает все источники присутствия"
    );
}

#[tokio::test]
async fn microphone_uplink_does_not_add_voice_connection() {
    let store = InMemoryVoicePresenceStore::default();
    let server_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    store
        .join(presence(
            Uuid::new_v4(),
            session_id,
            server_id,
            room_id,
            user_id,
        ))
        .await;
    let grant_id = Uuid::new_v4();
    store
        .issue_microphone_uplink_grant(MicrophoneUplinkGrant {
            id: grant_id,
            user_id,
            room_id,
            presence_session_id: session_id,
            expires_at: Utc::now() + Duration::seconds(20),
        })
        .await;
    store
        .consume_microphone_uplink_grant(&grant_id, &user_id, Uuid::new_v4(), Utc::now())
        .await
        .expect("отдельная сессия микрофона привязывается");

    assert_eq!(
        store.active_voice_connection_count().await,
        1,
        "отдельное подключение микрофона не считается ещё одним участником"
    );
}

#[tokio::test]
async fn replacing_presence_clears_video_sources_of_previous_session() {
    let store = InMemoryVoicePresenceStore::default();
    let server_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let old_session = Uuid::new_v4();
    let new_session = Uuid::new_v4();

    store
        .join(presence(
            Uuid::new_v4(),
            old_session,
            server_id,
            room_id,
            user_id,
        ))
        .await;
    store
        .inspect_video_datagram(
            old_session,
            &key_frame_datagram(MediaDatagramKind::CameraFrame, room_id),
            BASE_CAMERA_VIDEO_PRESETS,
        )
        .await;
    assert_eq!(store.active_video_source_count().await, 1);

    store
        .join(presence(
            Uuid::new_v4(),
            new_session,
            server_id,
            room_id,
            user_id,
        ))
        .await;

    assert_eq!(
        store.active_video_source_count().await,
        0,
        "вытеснение прежнего присутствия убирает его видеоисточники"
    );
    assert_eq!(store.active_voice_connection_count().await, 1);
}
