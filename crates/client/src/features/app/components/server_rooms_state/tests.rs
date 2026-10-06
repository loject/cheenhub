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
