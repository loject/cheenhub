//! Проверки привязки остановки системного звука к конкретному захвату.

use super::*;
use uuid::Uuid;

#[tokio::test]
async fn stopping_video_rejects_nil_or_camera_audio_identity() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let stream_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;
    join_room(
        &state,
        stream_id,
        session_id,
        &user,
        &user_id,
        JoinVoiceRoom {
            microphone_enabled: true,
            server_id: server_id.clone(),
            room_id: room_id.clone(),
        },
    )
    .await
    .unwrap();

    for (source, audio_track_id) in [
        (VoiceVideoStreamSource::Camera, Uuid::new_v4()),
        (VoiceVideoStreamSource::ScreenShare, Uuid::nil()),
    ] {
        let error = stop_video_stream(
            &state,
            stream_id,
            session_id,
            &user_id,
            StopVoiceVideoStream {
                server_id: server_id.clone(),
                room_id: room_id.clone(),
                source,
                audio_track_id: Some(audio_track_id),
            },
        )
        .await
        .unwrap_err();

        assert!(
            matches!(error, VoiceChatApplicationError::BadRequest(_)),
            "{source:?}"
        );
    }
}
