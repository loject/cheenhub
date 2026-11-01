//! Регрессии приоритета рингтонов личного звонка и голосового приглашения.

use super::desired_room_invite_ring;

#[test]
fn ignored_direct_call_allows_pending_voice_room_invite_to_ring() {
    let invite = Some("invite-1".to_owned());

    assert_eq!(desired_room_invite_ring(invite.clone(), true), None);
    assert_eq!(desired_room_invite_ring(invite.clone(), false), invite);
}

#[test]
fn direct_call_ring_preempts_voice_room_invite_and_no_invite_stays_silent() {
    assert_eq!(
        desired_room_invite_ring(Some("invite-1".to_owned()), true),
        None
    );
    assert_eq!(desired_room_invite_ring(None, false), None);
}
