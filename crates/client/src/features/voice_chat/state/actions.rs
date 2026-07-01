//! Realtime-действия, привязанные к цели голосового подключения.

use cheenhub_contracts::realtime::{VoiceRoomParticipant, VoiceRoomSnapshot};
use cheenhub_contracts::rest::AuthUser;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

use super::target::{VoiceRoomTarget, VoiceRoomTargetKind};
use crate::features::voice_chat::realtime;

pub(super) async fn join_target(
    realtime: &RealtimeHandle,
    target: &VoiceRoomTarget,
) -> Result<VoiceRoomSnapshot, RealtimeError> {
    match target.kind {
        VoiceRoomTargetKind::Server => {
            realtime::join_room(realtime, target.server_id.clone(), target.room_id.clone()).await
        }
        VoiceRoomTargetKind::DirectMessage => {
            realtime::join_direct_message_room(realtime, target.room_id.clone()).await
        }
    }
}

pub(super) async fn leave_target(
    realtime: &RealtimeHandle,
    target: &VoiceRoomTarget,
) -> Result<VoiceRoomSnapshot, RealtimeError> {
    match target.kind {
        VoiceRoomTargetKind::Server => {
            realtime::leave_room(realtime, target.server_id.clone(), target.room_id.clone()).await
        }
        VoiceRoomTargetKind::DirectMessage => {
            realtime::leave_direct_message_room(realtime, target.room_id.clone()).await
        }
    }
}

pub(super) fn ensure_current_user_present(
    participants: &mut Vec<VoiceRoomParticipant>,
    user: &AuthUser,
) {
    if participants
        .iter()
        .any(|participant| participant.user_id == user.id)
    {
        return;
    }

    participants.push(VoiceRoomParticipant {
        user_id: user.id.clone(),
        nickname: user.nickname.clone(),
        avatar_url: user.avatar_url.clone(),
        joined_at: String::new(),
    });
}
