use crate::Route;
use crate::features::voice_chat::VoiceRoomTarget;

use super::route_for_voice_target;

#[test]
fn opening_server_voice_target_navigates_to_its_room() {
    let target = VoiceRoomTarget::server(
        "server-1".to_owned(),
        "room-2".to_owned(),
        "Общая".to_owned(),
    );

    assert_eq!(
        route_for_voice_target(&target),
        Route::AppServerRoom {
            server_id: "server-1".to_owned(),
            room_id: "room-2".to_owned(),
        }
    );
}

#[test]
fn opening_direct_call_target_navigates_to_its_conversation() {
    let target =
        VoiceRoomTarget::direct_message("conversation-1".to_owned(), "Личный звонок".to_owned());

    assert_eq!(
        route_for_voice_target(&target),
        Route::AppDirectMessage {
            conversation_id: "conversation-1".to_owned(),
        }
    );
}
