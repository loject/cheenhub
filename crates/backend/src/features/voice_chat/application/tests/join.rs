//! Проверки ответа на присоединение к серверной голосовой комнате.

use cheenhub_contracts::realtime::JoinVoiceRoom;
use cheenhub_contracts::rest::ServerRoomKind;

use super::{create_room, join_room, registered_user, state};

#[tokio::test]
async fn joining_server_voice_room_returns_server_audio_bitrate() {
    let state = state();
    let (user, user_id) = registered_user(&state).await;
    let (server_id, room_id) = create_room(&state, &user_id, "voice", ServerRoomKind::Voice).await;
    let server_uuid = uuid::Uuid::parse_str(&server_id).expect("server id should be uuid");
    state
        .server_store
        .update_server_audio_bitrate(&server_uuid, &user_id, 48_000)
        .await
        .expect("bitrate should update");

    let snapshot = join_room(
        &state,
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        &user,
        &user_id,
        JoinVoiceRoom {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
        },
    )
    .await
    .expect("join should succeed");

    assert_eq!(snapshot.audio_bitrate_bps, Some(48_000));
    assert_eq!(snapshot.server_id, server_id);
    assert_eq!(snapshot.room_id, room_id);
}
