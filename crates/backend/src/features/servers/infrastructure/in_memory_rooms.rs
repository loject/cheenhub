//! In-memory room storage helpers.

use cheenhub_contracts::rest::ServerRoomKind;
use chrono::Utc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::features::servers::domain::ServerRoom;
use crate::features::servers::infrastructure::in_memory::InMemoryState;

pub(super) fn insert_server_room(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    name: String,
    kind: ServerRoomKind,
) -> anyhow::Result<ServerRoom> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    let position = state
        .rooms
        .iter()
        .filter(|room| room.server_id == *server_id)
        .map(|room| room.position)
        .max()
        .map(|position: u32| position.saturating_add(1))
        .unwrap_or(0);
    let now = Utc::now();
    let room = ServerRoom {
        id: Uuid::new_v4(),
        server_id: *server_id,
        name,
        kind,
        position,
        created_at: now,
        updated_at: now,
    };

    state.rooms.push(room.clone());

    Ok(room)
}

pub(super) fn list_server_rooms(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
) -> anyhow::Result<Vec<ServerRoom>> {
    let state = state.lock().map_err(|_| poisoned())?;
    let mut rooms = state
        .rooms
        .iter()
        .filter(|room| room.server_id == *server_id)
        .cloned()
        .collect::<Vec<_>>();
    rooms.sort_by_key(|room| room.position);

    Ok(rooms)
}

pub(super) fn find_server_room(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    room_id: &Uuid,
) -> anyhow::Result<Option<ServerRoom>> {
    let state = state.lock().map_err(|_| poisoned())?;

    Ok(state
        .rooms
        .iter()
        .find(|room| room.server_id == *server_id && room.id == *room_id)
        .cloned())
}

pub(super) fn update_server_room(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    room_id: &Uuid,
    name: String,
    kind: ServerRoomKind,
) -> anyhow::Result<Option<ServerRoom>> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    let Some(room) = state
        .rooms
        .iter_mut()
        .find(|room| room.server_id == *server_id && room.id == *room_id)
    else {
        return Ok(None);
    };

    room.name = name;
    room.kind = kind;
    room.updated_at = Utc::now();

    Ok(Some(room.clone()))
}

pub(super) fn delete_server_room(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
    room_id: &Uuid,
) -> anyhow::Result<()> {
    let mut state = state.lock().map_err(|_| poisoned())?;
    state
        .rooms
        .retain(|room| room.server_id != *server_id || room.id != *room_id);

    Ok(())
}

pub(super) fn count_server_rooms(
    state: &Mutex<InMemoryState>,
    server_id: &Uuid,
) -> anyhow::Result<u32> {
    let state = state.lock().map_err(|_| poisoned())?;

    Ok(state
        .rooms
        .iter()
        .filter(|room| room.server_id == *server_id)
        .count()
        .try_into()
        .unwrap_or(u32::MAX))
}

fn poisoned() -> anyhow::Error {
    anyhow::anyhow!("in-memory server store lock poisoned")
}
