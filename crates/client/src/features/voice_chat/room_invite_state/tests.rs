//! Регрессии синхронизации приглашений между устройствами.

use std::collections::HashSet;

use cheenhub_contracts::realtime::{VoiceRoomInviteSnapshot, VoiceRoomInviteStatus};

use super::{incoming_invite, upsert_invite};

fn invite(status: VoiceRoomInviteStatus) -> VoiceRoomInviteSnapshot {
    VoiceRoomInviteSnapshot {
        invite_id: "invite-1".to_owned(),
        server_id: "server-1".to_owned(),
        server_name: "Сервер".to_owned(),
        room_id: "room-1".to_owned(),
        room_name: "Голосовая".to_owned(),
        caller_user_id: "caller".to_owned(),
        caller_nickname: "Зовущий".to_owned(),
        caller_avatar_url: None,
        recipient_user_id: "recipient".to_owned(),
        status,
        created_at: "2026-10-31T15:00:00Z".to_owned(),
        expires_at: "2026-10-31T15:00:45Z".to_owned(),
    }
}

#[test]
fn invite_state_upserts_lifecycle_without_duplicate_ringing() {
    let mut invites = Vec::new();
    upsert_invite(&mut invites, invite(VoiceRoomInviteStatus::Pending));
    upsert_invite(&mut invites, invite(VoiceRoomInviteStatus::Accepted));
    assert_eq!(invites.len(), 1);
    assert_eq!(invites[0].status, VoiceRoomInviteStatus::Accepted);
    assert!(incoming_invite(&invites, &HashSet::new(), "recipient").is_none());
}

#[test]
fn invite_state_does_not_resurrect_terminal_after_delayed_send_ack() {
    for terminal_status in [
        VoiceRoomInviteStatus::Accepted,
        VoiceRoomInviteStatus::Declined,
        VoiceRoomInviteStatus::Expired,
        VoiceRoomInviteStatus::Cancelled,
    ] {
        let mut invites = vec![invite(terminal_status)];
        upsert_invite(&mut invites, invite(VoiceRoomInviteStatus::Pending));
        assert_eq!(invites.len(), 1);
        assert_eq!(invites[0].status, terminal_status);
        assert!(incoming_invite(&invites, &HashSet::new(), "recipient").is_none());
    }
}

#[test]
fn invite_ignore_hides_only_this_device_without_changing_server_state() {
    let invites = vec![invite(VoiceRoomInviteStatus::Pending)];
    let mut ignored = HashSet::new();
    assert!(incoming_invite(&invites, &ignored, "recipient").is_some());
    ignored.insert("invite-1".to_owned());
    assert!(incoming_invite(&invites, &ignored, "recipient").is_none());
    assert_eq!(invites[0].status, VoiceRoomInviteStatus::Pending);
    assert!(incoming_invite(&invites, &HashSet::new(), "recipient").is_some());
    assert!(incoming_invite(&invites, &HashSet::new(), "caller").is_none());
}
