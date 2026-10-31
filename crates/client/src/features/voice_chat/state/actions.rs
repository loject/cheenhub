//! Realtime-действия, привязанные к цели голосового подключения.

use cheenhub_contracts::media::VOICE_AUDIO_BITRATE_BPS;
use cheenhub_contracts::realtime::{RejectionCode, VoiceRoomParticipant, VoiceRoomSnapshot};
use cheenhub_contracts::rest::AuthUser;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

use super::target::{VoiceRoomTarget, VoiceRoomTargetKind};
use crate::features::voice_chat::realtime;

/// Результат входа, который применяется только после проверки актуальности операции.
pub(super) struct JoinedVoiceRoom {
    /// Снимок участников комнаты.
    pub(super) snapshot: VoiceRoomSnapshot,
    /// UUID основной голосовой WebTransport-сессии, подтверждённый сервером.
    pub(super) self_session_id: String,
    /// Целевой битрейт микрофона для выбранной комнаты.
    pub(super) audio_bitrate_bps: u32,
}

/// Отказывает в отправке медиа, если сервер не подтвердил реальную голосовую сессию.
pub(super) fn confirmed_voice_session_id(
    snapshot: &VoiceRoomSnapshot,
) -> Result<String, RealtimeError> {
    snapshot.self_session_id.clone().ok_or_else(|| {
        RealtimeError::rejected(
            RejectionCode::VoiceClientUpgradeRequired,
            "Сервер не поддерживает подтверждение голосовой сессии. Обновите CheenHub на сервере.",
        )
    })
}

pub(super) async fn join_target(
    realtime: &RealtimeHandle,
    target: &VoiceRoomTarget,
    microphone_enabled: bool,
    confirm_disconnect_other_devices: bool,
) -> Result<JoinedVoiceRoom, RealtimeError> {
    let (response, audio_bitrate_bps) = match target.kind {
        VoiceRoomTargetKind::Server => {
            let response = realtime::join_room(
                realtime,
                target.server_id.clone(),
                target.room_id.clone(),
                microphone_enabled,
                confirm_disconnect_other_devices,
            )
            .await?;
            let bitrate = response
                .audio_bitrate_bps
                .unwrap_or(VOICE_AUDIO_BITRATE_BPS);
            (response, bitrate)
        }
        VoiceRoomTargetKind::DirectMessage => {
            let response = realtime::join_direct_message_room(
                realtime,
                target.room_id.clone(),
                microphone_enabled,
                confirm_disconnect_other_devices,
            )
            .await?;
            (response, VOICE_AUDIO_BITRATE_BPS)
        }
    };
    let self_session_id = match confirmed_voice_session_id(&response) {
        Ok(session_id) => session_id,
        Err(error) => {
            // Старый сервер уже мог записать присутствие: отменяем этот вход.
            if let Err(leave_error) = leave_target(realtime, target).await {
                dioxus::logger::tracing::warn!(
                    %leave_error,
                    server_id = %target.server_id,
                    room_id = %target.room_id,
                    "failed to remove voice presence after unsupported server join"
                );
            }
            return Err(error);
        }
    };
    Ok(JoinedVoiceRoom {
        snapshot: response,
        self_session_id,
        audio_bitrate_bps,
    })
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
    microphone_enabled: bool,
    self_session_id: &str,
) {
    if participants.iter().any(|participant| {
        participant.user_id == user.id && participant.session_id == self_session_id
    }) {
        return;
    }

    participants.push(VoiceRoomParticipant {
        microphone_enabled,
        output_muted: false,
        session_id: self_session_id.to_owned(),
        voice_protocol_version: 6,
        device_kind: realtime::voice_device_kind(),
        user_id: user.id.clone(),
        nickname: user.nickname.clone(),
        avatar_url: user.avatar_url.clone(),
        joined_at: String::new(),
    });
}
