//! Проверки валидации входных данных сервера.

use cheenhub_contracts::rest::{ServerRoomWriteAccess, ServerRoomWriteAccessMode};
use uuid::Uuid;

use super::{ValidatedWriteAccess, create_server};

#[test]
fn trims_valid_server_name() {
    let valid = create_server("  CheenHub Dev  ".to_owned()).expect("name should be valid");

    assert_eq!(valid.name, "CheenHub Dev");
}

#[test]
fn rejects_empty_server_name() {
    assert!(create_server("   ".to_owned()).is_err());
}

#[test]
fn rejects_short_server_name() {
    assert!(create_server("a".to_owned()).is_err());
}

#[test]
fn rejects_long_server_name() {
    assert!(create_server("a".repeat(49)).is_err());
}

#[test]
fn trims_valid_room_name() {
    let valid = super::server_room("  x  ".to_owned(), ServerRoomWriteAccess::default())
        .expect("room name should be valid");

    assert_eq!(valid.name, "x");
}

#[test]
fn rejects_empty_room_name() {
    assert!(super::server_room("   ".to_owned(), ServerRoomWriteAccess::default()).is_err());
}

#[test]
fn rejects_long_room_name() {
    assert!(super::server_room("a".repeat(49), ServerRoomWriteAccess::default()).is_err());
}

#[test]
fn room_write_access_defaults_to_all_members() {
    let valid = super::server_room("общий".to_owned(), ServerRoomWriteAccess::default())
        .expect("room should be valid");

    assert_eq!(valid.write_access, ValidatedWriteAccess::all_members());
}

#[test]
fn clears_room_write_roles_in_all_members_mode() {
    let valid = super::server_room(
        "общий".to_owned(),
        ServerRoomWriteAccess {
            mode: ServerRoomWriteAccessMode::AllMembers,
            role_ids: vec![Uuid::new_v4().to_string()],
        },
    )
    .expect("room should be valid");

    assert_eq!(valid.write_access, ValidatedWriteAccess::all_members());
}

#[test]
fn rejects_malformed_room_write_role() {
    let result = super::server_room(
        "курилка".to_owned(),
        ServerRoomWriteAccess {
            mode: ServerRoomWriteAccessMode::SelectedRoles,
            role_ids: vec!["не-uuid".to_owned()],
        },
    );

    assert!(result.is_err());
}

#[test]
fn deduplicates_room_write_roles() {
    let role_id = Uuid::new_v4();
    let valid = super::server_room(
        "курилка".to_owned(),
        ServerRoomWriteAccess {
            mode: ServerRoomWriteAccessMode::SelectedRoles,
            role_ids: vec![role_id.to_string(), role_id.to_string()],
        },
    )
    .expect("room should be valid");

    assert_eq!(
        valid.write_access.mode,
        ServerRoomWriteAccessMode::SelectedRoles
    );
    assert_eq!(valid.write_access.role_ids, vec![role_id]);
}

#[test]
fn accepts_valid_invite_settings() {
    let valid =
        super::create_server_invite(Some(30), Some(7)).expect("invite settings should be valid");

    assert_eq!(valid.max_uses, Some(30));
    assert_eq!(valid.expires_in_days, Some(7));
}

#[test]
fn rejects_invalid_invite_usage_limit() {
    assert!(super::create_server_invite(Some(0), None).is_err());
    assert!(super::create_server_invite(Some(1000), None).is_err());
}

#[test]
fn rejects_invalid_invite_expiration() {
    assert!(super::create_server_invite(None, Some(0)).is_err());
    assert!(super::create_server_invite(None, Some(366)).is_err());
}
