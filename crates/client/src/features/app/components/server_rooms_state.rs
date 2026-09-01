//! Server room workspace state helpers.

use cheenhub_contracts::rest::{ServerRoomKind, ServerRoomSummary};
use dioxus::prelude::{Signal, WritableExt};

use super::app_shell::ActiveRoom;

#[derive(Clone, PartialEq)]
pub(super) enum RoomModal {
    Create,
    Edit(ServerRoomSummary),
}

#[derive(Clone, PartialEq, Eq)]
pub(super) enum ServerWorkspace {
    Room(String),
    Settings,
}

pub(super) fn ensure_workspace_mounted(
    workspaces: &mut Vec<ServerWorkspace>,
    workspace: ServerWorkspace,
) -> bool {
    // TODO: Vec::contains линейный, но для UI-workspace это приемлемо.
    // Если список вырастет, заменить на IndexSet.
    if !workspaces.contains(&workspace) {
        workspaces.push(workspace);
        return true;
    }

    false
}

pub(super) fn clear_workspace_selection_if_needed(
    mut active_workspace: Signal<Option<ServerWorkspace>>,
    mut reported_room_id: Signal<Option<String>>,
) {
    let current_workspace = active_workspace();
    if matches!(current_workspace, Some(ServerWorkspace::Settings)) {
        if reported_room_id().is_some() {
            reported_room_id.set(None);
        }
        return;
    }
    if current_workspace.is_some() || reported_room_id().is_some() {
        active_workspace.set(None);
        reported_room_id.set(None);
    }
}

/// Возвращает, может ли синхронизация маршрута активировать workspace комнаты.
pub(super) fn should_activate_room_workspace(active_workspace: Option<&ServerWorkspace>) -> bool {
    !matches!(active_workspace, Some(ServerWorkspace::Settings))
}

pub(super) fn mount_workspace_if_missing(
    mut mounted_workspaces: Signal<Vec<ServerWorkspace>>,
    workspace: ServerWorkspace,
) {
    let mut next_workspaces = mounted_workspaces();
    if ensure_workspace_mounted(&mut next_workspaces, workspace) {
        mounted_workspaces.set(next_workspaces);
    }
}

pub(super) fn set_active_workspace_if_needed(
    mut active_workspace: Signal<Option<ServerWorkspace>>,
    workspace: ServerWorkspace,
) {
    if active_workspace() != Some(workspace.clone()) {
        active_workspace.set(Some(workspace));
    }
}

/// Открывает параметры сервера и сохраняет workspace смонтированным.
pub(super) fn open_server_settings_workspace(
    mut mounted_workspaces: Signal<Vec<ServerWorkspace>>,
    mut active_workspace: Signal<Option<ServerWorkspace>>,
    mut mobile_workspace_open: Signal<bool>,
) {
    let workspace = ServerWorkspace::Settings;
    let mut next_mounted_workspaces = mounted_workspaces();
    ensure_workspace_mounted(&mut next_mounted_workspaces, workspace.clone());
    mounted_workspaces.set(next_mounted_workspaces);
    active_workspace.set(Some(workspace));
    mobile_workspace_open.set(true);
}

/// Закрывает параметры сервера и восстанавливает выбранную комнату, если она есть.
pub(super) fn close_server_settings_workspace(
    active_room_id: Option<String>,
    mut mounted_workspaces: Signal<Vec<ServerWorkspace>>,
    mut active_workspace: Signal<Option<ServerWorkspace>>,
) {
    let Some(room_id) = active_room_id else {
        active_workspace.set(None);
        return;
    };
    let workspace = ServerWorkspace::Room(room_id);
    let mut next_mounted_workspaces = mounted_workspaces();
    ensure_workspace_mounted(&mut next_mounted_workspaces, workspace.clone());
    mounted_workspaces.set(next_mounted_workspaces);
    active_workspace.set(Some(workspace));
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

pub(super) fn resolve_active_room_id(
    rooms: &[ServerRoomSummary],
    requested_room_id: Option<&str>,
    current_room_id: Option<&str>,
) -> Option<String> {
    requested_room_id
        .filter(|room_id| rooms.iter().any(|room| room.id == *room_id))
        .or_else(|| current_room_id.filter(|room_id| rooms.iter().any(|room| room.id == *room_id)))
        .or_else(|| rooms.first().map(|room| room.id.as_str()))
        .map(ToOwned::to_owned)
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

#[cfg(test)]
mod tests {
    use super::{ServerWorkspace, should_activate_room_workspace};

    #[test]
    fn room_sync_preserves_open_server_settings() {
        assert!(!should_activate_room_workspace(Some(
            &ServerWorkspace::Settings
        )));
    }

    #[test]
    fn room_sync_selects_room_without_another_workspace() {
        assert!(should_activate_room_workspace(None));
        assert!(should_activate_room_workspace(Some(
            &ServerWorkspace::Room("room-id".to_owned())
        )));
    }
}
