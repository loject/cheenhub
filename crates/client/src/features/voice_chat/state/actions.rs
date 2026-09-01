//! Realtime-действия, привязанные к цели голосового подключения.

use cheenhub_contracts::media::VOICE_AUDIO_BITRATE_BPS;
use cheenhub_contracts::realtime::{VoiceRoomParticipant, VoiceRoomSnapshot};
use cheenhub_contracts::rest::AuthUser;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

use super::target::{VoiceRoomTarget, VoiceRoomTargetKind};
use crate::features::voice_chat::realtime;

/// Результат входа, который применяется только после проверки актуальности операции.
pub(super) struct JoinedVoiceRoom {
    /// Снимок участников комнаты.
    pub(super) snapshot: VoiceRoomSnapshot,
    /// Целевой битрейт микрофона для выбранной комнаты.
    pub(super) audio_bitrate_bps: u32,
}

pub(super) async fn join_target(
    realtime: &RealtimeHandle,
    target: &VoiceRoomTarget,
) -> Result<JoinedVoiceRoom, RealtimeError> {
    match target.kind {
        VoiceRoomTargetKind::Server => {
            let snapshot =
                realtime::join_room(realtime, target.server_id.clone(), target.room_id.clone())
                    .await?;
            let audio_bitrate_bps = snapshot
                .audio_bitrate_bps
                .unwrap_or(VOICE_AUDIO_BITRATE_BPS);
            Ok(JoinedVoiceRoom {
                snapshot,
                audio_bitrate_bps,
            })
        }
        VoiceRoomTargetKind::DirectMessage => {
            let snapshot =
                realtime::join_direct_message_room(realtime, target.room_id.clone()).await?;
            Ok(JoinedVoiceRoom {
                snapshot,
                audio_bitrate_bps: VOICE_AUDIO_BITRATE_BPS,
            })
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
