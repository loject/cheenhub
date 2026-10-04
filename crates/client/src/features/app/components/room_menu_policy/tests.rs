use super::{RoomMenuCommand, available_commands};

#[test]
fn missing_permission_hides_all_commands_for_room_and_empty_space() {
    for room_target in [false, true] {
        assert!(available_commands(false, false, room_target, true, false).is_empty());
    }
}

#[test]
fn empty_space_offers_creation_only_after_loading() {
    assert_eq!(
        available_commands(true, false, false, false, false),
        &[RoomMenuCommand::Create]
    );
    assert!(available_commands(true, true, false, false, false).is_empty());
}

#[test]
fn existing_room_offers_edit_and_delete() {
    assert_eq!(
        available_commands(true, false, true, true, false),
        &[RoomMenuCommand::Edit, RoomMenuCommand::Delete]
    );
}

#[test]
fn removed_or_deleting_room_cannot_be_modified_or_replaced_with_creation() {
    assert!(available_commands(true, false, true, false, false).is_empty());
    assert!(available_commands(true, false, true, true, true).is_empty());
}
