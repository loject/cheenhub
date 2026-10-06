use super::{next_active_room_after_delete, workspace_after_room_delete};
use crate::features::app::components::server_rooms_state::ServerWorkspace;
use cheenhub_contracts::rest::ServerRoomSummary;

fn room(id: &str) -> ServerRoomSummary {
    ServerRoomSummary {
        id: id.to_owned(),
        name: id.to_owned(),
        kind: cheenhub_contracts::rest::ServerRoomKind::Text,
        position: 0,
        write_access: Default::default(),
        can_write: true,
    }
}

#[test]
fn deleting_active_room_selects_first_remaining_room() {
    let remaining = vec![room("first"), room("second")];

    let next = next_active_room_after_delete(&remaining, "first", Some("first"));

    assert_eq!(next.as_deref(), Some("first"));
}

#[test]
fn deleting_other_room_keeps_current_selection() {
    let remaining = vec![room("first")];

    let next = next_active_room_after_delete(&remaining, "other", Some("first"));

    assert_eq!(next.as_deref(), Some("first"));
}

#[test]
fn deleting_last_room_clears_active_workspace() {
    let mounted = vec![ServerWorkspace::Room("first".to_owned())];
    let active = Some(ServerWorkspace::Room("first".to_owned()));

    let (next_mounted, next_active) = workspace_after_room_delete(mounted, active, "first", None);

    assert!(next_mounted.is_empty());
    assert_eq!(next_active, None);
}

#[test]
fn deleting_inactive_room_keeps_active_workspace() {
    let mounted = vec![
        ServerWorkspace::Room("first".to_owned()),
        ServerWorkspace::Room("second".to_owned()),
    ];
    let active = Some(ServerWorkspace::Room("first".to_owned()));

    let (next_mounted, next_active) =
        workspace_after_room_delete(mounted, active, "second", Some("first"));

    assert_eq!(
        next_mounted,
        vec![ServerWorkspace::Room("first".to_owned())]
    );
    assert_eq!(next_active, Some(ServerWorkspace::Room("first".to_owned())));
}
