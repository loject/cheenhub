//! Регрессии отображения состояния подключения рядом с приглашениями.

use super::{VoiceParticipantGridStatus, should_show_connection_state};

#[test]
fn failed_connection_keeps_retry_visible_with_outgoing_invite_tile() {
    let status = VoiceParticipantGridStatus::Error {
        message: "Не удалось подключиться".to_owned(),
    };

    assert!(should_show_connection_state(&status, true, false));
}

#[test]
fn connecting_room_keeps_loading_state_with_outgoing_invite_tile() {
    assert!(should_show_connection_state(
        &VoiceParticipantGridStatus::Connecting,
        true,
        false,
    ));
}

#[test]
fn connected_empty_room_can_display_outgoing_invite_tile() {
    assert!(!should_show_connection_state(
        &VoiceParticipantGridStatus::Empty,
        true,
        false,
    ));
    assert!(should_show_connection_state(
        &VoiceParticipantGridStatus::Empty,
        true,
        true,
    ));
}
