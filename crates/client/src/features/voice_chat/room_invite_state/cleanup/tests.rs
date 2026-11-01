//! Регрессии удаления завершённых приглашений из локального состояния.

use cheenhub_contracts::realtime::VoiceRoomInviteStatus;

use super::remove_terminal_invite;
use crate::features::voice_chat::room_invite_state::tests::invite;

#[test]
fn cleanup_removes_terminal_without_touching_other_or_pending_invites() {
    let mut completed = invite(VoiceRoomInviteStatus::Accepted);
    completed.invite_id = "completed".to_owned();
    let mut other = invite(VoiceRoomInviteStatus::Declined);
    other.invite_id = "other".to_owned();
    let mut pending = invite(VoiceRoomInviteStatus::Pending);
    pending.invite_id = "pending".to_owned();
    let mut invites = vec![completed, other.clone(), pending.clone()];

    assert!(remove_terminal_invite(&mut invites, "completed"));
    assert!(!remove_terminal_invite(&mut invites, "pending"));
    assert!(!remove_terminal_invite(&mut invites, "unknown"));
    assert_eq!(invites, vec![other, pending]);
}
