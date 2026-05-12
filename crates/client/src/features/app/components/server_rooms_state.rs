//! Server room workspace state helpers.

use cheenhub_contracts::rest::{ServerRoomKind, ServerRoomSummary};

use super::app_shell::ActiveRoom;

#[derive(Clone, PartialEq, Eq)]
pub(super) enum ServerWorkspace {
    Room(String),
    Settings,
}

pub(super) fn ensure_workspace_mounted(
    workspaces: &mut Vec<ServerWorkspace>,
    workspace: ServerWorkspace,
) {
    // TODO: Vec::contains is linear, which is fine for UI workspaces. Use IndexSet if
    // this grows into many tabs or frequently mounted workspace kinds.
    if !workspaces.contains(&workspace) {
        workspaces.push(workspace);
    }
}

pub(super) fn active_room(
    rooms: &[ServerRoomSummary],
    active_room_id: Option<&str>,
) -> Option<ActiveRoom> {
    let room = active_room_id
        .and_then(|room_id| rooms.iter().find(|room| room.id == room_id))
        .or_else(|| rooms.first())?;

    Some(ActiveRoom {
        id: room.id.clone(),
        kind: room.kind,
        name: room.name.clone(),
    })
}

pub(super) fn room_by_id(rooms: &[ServerRoomSummary], room_id: &str) -> Option<ActiveRoom> {
    let room = rooms.iter().find(|room| room.id == room_id)?;

    Some(ActiveRoom {
        id: room.id.clone(),
        kind: room.kind,
        name: room.name.clone(),
    })
}

pub(super) fn chat_open_for_room(chat_open_by_room: &[(String, bool)], room_id: &str) -> bool {
    chat_open_by_room
        .iter()
        .find_map(|(saved_room_id, chat_open)| (saved_room_id == room_id).then_some(*chat_open))
        .unwrap_or(false)
}

pub(super) fn upsert_room(rooms: &mut Vec<ServerRoomSummary>, room: ServerRoomSummary) {
    if let Some(saved_room) = rooms.iter_mut().find(|saved_room| saved_room.id == room.id) {
        *saved_room = room;
        return;
    }

    rooms.push(room);
}

pub(super) fn room_icon(kind: ServerRoomKind) -> &'static str {
    match kind {
        ServerRoomKind::Text => "#",
        ServerRoomKind::Voice => "~",
        ServerRoomKind::TextAndVoice => "&",
    }
}

pub(super) fn room_icon_class(kind: ServerRoomKind) -> &'static str {
    match kind {
        ServerRoomKind::Text => "w-3.5 shrink-0 text-center text-zinc-600",
        ServerRoomKind::Voice => {
            "w-3.5 shrink-0 text-center text-[13px] font-semibold leading-none text-zinc-500"
        }
        ServerRoomKind::TextAndVoice => {
            "w-3.5 shrink-0 text-center text-[13px] font-semibold leading-none text-accent"
        }
    }
}
