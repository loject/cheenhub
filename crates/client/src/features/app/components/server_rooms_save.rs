//! Применение результата редактора комнаты к состоянию сайдбара.

use cheenhub_contracts::rest::ServerRoomSummary;
use dioxus::prelude::*;
use dioxus::router::Navigator;

use crate::Route;

use super::app_shell::{ServerShellState, room_kind_attr};
use super::server_rooms_delete::RoomSidebarContext;
use super::server_rooms_state::{
    ServerWorkspace, chat_open_for_room, ensure_workspace_mounted, upsert_room,
};

/// Добавляет сохранённую комнату в список, открывает её и переводит маршрут.
///
/// Список комнат хранится в ресурсе загрузки, поэтому комната записывается прямо
/// в ресурс: это избавляет от повторного запроса сразу после сохранения и сохраняет
/// порядок комнат по позиции, который вернул сервер.
pub(super) fn apply_saved_room(
    server_id: String,
    saved_room: ServerRoomSummary,
    context: RoomSidebarContext,
    mut mobile_workspace_open: Signal<bool>,
    mut room_action_status: Signal<String>,
    navigator: Navigator,
) {
    let RoomSidebarContext {
        mut rooms,
        mut active_room_id,
        mut mounted_workspaces,
        mut active_workspace,
        chat_open_by_room,
        on_state_change,
    } = context;
    info!(
        %server_id,
        room_id = %saved_room.id,
        room_kind = ?saved_room.kind,
        "saved server room from sidebar editor"
    );

    let mut next_rooms = match rooms.read().clone() {
        Some(Ok(rooms)) => rooms,
        _ => Vec::new(),
    };
    upsert_room(&mut next_rooms, saved_room.clone());
    next_rooms.sort_by_key(|room| room.position);
    *rooms.write() = Some(Ok(next_rooms));

    active_room_id.set(Some(saved_room.id.clone()));
    let workspace = ServerWorkspace::Room(saved_room.id.clone());
    let mut next_mounted_workspaces = mounted_workspaces();
    ensure_workspace_mounted(&mut next_mounted_workspaces, workspace.clone());
    mounted_workspaces.set(next_mounted_workspaces);
    active_workspace.set(Some(workspace));
    mobile_workspace_open.set(true);
    room_action_status.set(String::new());

    on_state_change.call((
        server_id.clone(),
        ServerShellState {
            chat_open: chat_open_for_room(&chat_open_by_room(), &saved_room.id),
            room_kind: room_kind_attr(saved_room.kind),
        },
    ));
    navigator.push(Route::AppServerRoom {
        server_id,
        room_id: saved_room.id,
    });
}
