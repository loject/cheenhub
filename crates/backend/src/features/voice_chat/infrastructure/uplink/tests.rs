//! Проверки выдачи и потребления грантов микрофона.

use chrono::{Duration, Utc};
use uuid::Uuid;

use super::{ConsumeMicrophoneUplinkGrantError, MicrophoneUplinkGrant};
use crate::features::voice_chat::infrastructure::{
    InMemoryVoicePresenceStore, VoicePresence, VoicePresenceTargetKind,
};

#[tokio::test]
async fn grant_is_one_time_and_bound_to_user() {
    let store = InMemoryVoicePresenceStore::default();
    let grant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let presence_session_id = Uuid::new_v4();
    let worker_session_id = Uuid::new_v4();
    store
        .issue_microphone_uplink_grant(MicrophoneUplinkGrant {
            id: grant_id,
            user_id,
            room_id,
            presence_session_id,
            expires_at: Utc::now() + Duration::seconds(20),
        })
        .await;

    let wrong_user_error = store
        .consume_microphone_uplink_grant(&grant_id, &Uuid::new_v4(), worker_session_id, Utc::now())
        .await
        .expect_err("grant другого пользователя должен быть отклонен");
    assert_eq!(wrong_user_error, ConsumeMicrophoneUplinkGrantError::Invalid);
    store
        .consume_microphone_uplink_grant(&grant_id, &user_id, worker_session_id, Utc::now())
        .await
        .expect("владелец должен потребить grant");
    assert!(
        store
            .microphone_uplink_is_bound(
                &worker_session_id,
                &user_id,
                &room_id,
                &presence_session_id,
            )
            .await
    );
    assert_eq!(
        store
            .consume_microphone_uplink_grant(&grant_id, &user_id, Uuid::new_v4(), Utc::now())
            .await
            .expect_err("grant нельзя использовать повторно"),
        ConsumeMicrophoneUplinkGrantError::Invalid
    );
}

#[tokio::test]
async fn replacement_grant_revokes_previous_worker_binding() {
    let store = InMemoryVoicePresenceStore::default();
    let user_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let presence_session_id = Uuid::new_v4();
    let worker_session_id = Uuid::new_v4();
    let first_grant_id = Uuid::new_v4();
    store
        .issue_microphone_uplink_grant(MicrophoneUplinkGrant {
            id: first_grant_id,
            user_id,
            room_id,
            presence_session_id,
            expires_at: Utc::now() + Duration::seconds(20),
        })
        .await;
    store
        .consume_microphone_uplink_grant(&first_grant_id, &user_id, worker_session_id, Utc::now())
        .await
        .expect("первый grant должен привязать worker");

    store
        .issue_microphone_uplink_grant(MicrophoneUplinkGrant {
            id: Uuid::new_v4(),
            user_id,
            room_id,
            presence_session_id,
            expires_at: Utc::now() + Duration::seconds(20),
        })
        .await;

    assert!(
        !store
            .microphone_uplink_is_bound(
                &worker_session_id,
                &user_id,
                &room_id,
                &presence_session_id,
            )
            .await
    );
}

#[tokio::test]
async fn leaving_presence_revokes_bound_uplink() {
    let store = InMemoryVoicePresenceStore::default();
    let realtime_stream_id = Uuid::new_v4();
    let presence_session_id = Uuid::new_v4();
    let server_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let worker_session_id = Uuid::new_v4();
    store
        .join(VoicePresence {
            realtime_stream_id,
            session_id: presence_session_id,
            target_kind: VoicePresenceTargetKind::Server,
            server_id,
            room_id,
            user_id,
            nickname: "voice_user".to_owned(),
            avatar_url: None,
            joined_at: Utc::now(),
        })
        .await;
    let grant_id = Uuid::new_v4();
    store
        .issue_microphone_uplink_grant(MicrophoneUplinkGrant {
            id: grant_id,
            user_id,
            room_id,
            presence_session_id,
            expires_at: Utc::now() + Duration::seconds(20),
        })
        .await;
    store
        .consume_microphone_uplink_grant(&grant_id, &user_id, worker_session_id, Utc::now())
        .await
        .expect("grant должен привязать worker");

    store.leave_realtime_stream(&realtime_stream_id).await;

    assert!(
        !store
            .microphone_uplink_is_bound(
                &worker_session_id,
                &user_id,
                &room_id,
                &presence_session_id,
            )
            .await
    );
}

#[tokio::test]
async fn expired_grant_cannot_bind_uplink() {
    let store = InMemoryVoicePresenceStore::default();
    let grant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    store
        .issue_microphone_uplink_grant(MicrophoneUplinkGrant {
            id: grant_id,
            user_id,
            room_id: Uuid::new_v4(),
            presence_session_id: Uuid::new_v4(),
            expires_at: Utc::now() - Duration::seconds(1),
        })
        .await;

    assert_eq!(
        store
            .consume_microphone_uplink_grant(&grant_id, &user_id, Uuid::new_v4(), Utc::now(),)
            .await
            .expect_err("истекший grant должен быть отклонен"),
        ConsumeMicrophoneUplinkGrantError::Expired
    );
}
