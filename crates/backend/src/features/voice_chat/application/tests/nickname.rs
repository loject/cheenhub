//! Voice nickname update tests.

use crate::features::voice_chat::infrastructure::VoicePresenceTargetKind;
use cheenhub_contracts::realtime::JoinVoiceRoom;
use cheenhub_contracts::rest::ServerRoomKind;

use super::{create_room, registered_user, state};
use crate::features::voice_chat::application::{join_room, update_user_nickname};

#[tokio::test]
async fn nickname_update_changes_active_voice_presence() {
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
        JoinVoiceRoom {
            server_id: server_id.clone(),
            room_id: room_id.clone(),
        },
    )
    .await
    .expect("join should succeed");

    update_user_nickname(&state, &user_id, "renamed_voice".to_owned()).await;

    let server_uuid = uuid::Uuid::parse_str(&server_id).expect("server id should parse");
    let room_uuid = uuid::Uuid::parse_str(&room_id).expect("room id should parse");
    let participants = state
        .voice_presence_store
        .room_participants(VoicePresenceTargetKind::Server, &server_uuid, &room_uuid)
        .await;
    assert_eq!(participants[0].nickname, "renamed_voice");
}
