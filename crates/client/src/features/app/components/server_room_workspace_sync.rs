//! Синхронизация выбранной комнаты с активным workspace сервера.

use dioxus::prelude::*;
use dioxus::router::Navigator;

use crate::Route;

use super::server_rooms_state::{
    ServerWorkspace, mount_workspace_if_missing, set_active_workspace_if_needed,
    should_activate_room_workspace,
};

/// Активирует workspace комнаты, не вытесняя открытые параметры сервера.
pub(super) fn synchronize_room_workspace(
    active: bool,
    requested_room_id: Option<&str>,
    server_id: &str,
    room_id: &str,
    navigator: &Navigator,
    mounted_workspaces: Signal<Vec<ServerWorkspace>>,
    active_workspace: Signal<Option<ServerWorkspace>>,
) {
    let activate_room_workspace = {
        let current_workspace = active_workspace.peek();
        should_activate_room_workspace(current_workspace.as_ref())
    };
    if !activate_room_workspace {
        debug!(
            server_id,
            room_id, "preserving server settings workspace during room synchronization"
        );
        return;
    }

    let workspace = ServerWorkspace::Room(room_id.to_owned());
    mount_workspace_if_missing(mounted_workspaces, workspace.clone());
    set_active_workspace_if_needed(active_workspace, workspace);

    if active && requested_room_id != Some(room_id) {
        info!(
            server_id,
            room_id, "replacing server workspace route with resolved room"
        );
        navigator.replace(Route::AppServerRoom {
            server_id: server_id.to_owned(),
            room_id: room_id.to_owned(),
        });
    }
}
