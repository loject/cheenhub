//! Text chat event delivery policy.

use cheenhub_contracts::rest::ServerRoomKind;
use uuid::Uuid;

use crate::state::AppState;

/// Returns whether a user can receive room-scoped text chat events.
pub(crate) async fn can_receive_room_event(
    state: &AppState,
    user_id: &Uuid,
    server_id: &Uuid,
    room_id: &Uuid,
) -> anyhow::Result<bool> {
    let Some(room) = state
        .server_store
        .find_server_room(server_id, room_id)
        .await?
    else {
        return Ok(false);
    };
    if room.kind == ServerRoomKind::Voice {
        return Ok(false);
    }

    let Some(server) = state.server_store.find_server(server_id).await? else {
        return Ok(false);
    };
    if server.owner_user_id == *user_id {
        return Ok(true);
    }

    Ok(state
        .server_store
        .find_active_server_member(server_id, user_id)
        .await?
        .is_some())
}
