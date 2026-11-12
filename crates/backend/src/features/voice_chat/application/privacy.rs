//! Завершение голосовых взаимодействий после блокировки пользователей.

use cheenhub_contracts::realtime::{
    RealtimeKind, RealtimeModule, VoiceChatKind, VoiceNetworkTargetKind, VoicePresenceEvicted,
    VoicePresenceEvictionReason, VoiceRoomSnapshot,
};
use chrono::Utc;
use uuid::Uuid;

use super::{
    VoiceChatApplicationError, direct_calls, ensure_direct_message_voice_available, room_invites,
    social_error,
};
use crate::features::social::{self, DirectMessageVoiceAccess};
use crate::state::AppState;

/// Повторно проверяет право входа в личное медиа под блокировкой операций пары.
///
/// Guard удерживается вызывающим кодом до завершения регистрации присутствия.
pub(super) async fn authorize_direct_media_join<'a>(
    state: &'a AppState,
    user_id: &Uuid,
    conversation_id: &Uuid,
) -> Result<(DirectMessageVoiceAccess, tokio::sync::MutexGuard<'a, ()>), VoiceChatApplicationError>
{
    let access = ensure_direct_message_voice_available(state, user_id, conversation_id).await?;
    let guard = state
        .contact_pair_guards
        .lock(user_id, &access.peer_user_id)
        .await;
    let access = ensure_direct_message_voice_available(state, user_id, conversation_id).await?;
    social::ensure_incoming_call_allowed(state, user_id, &access.peer_user_id)
        .await
        .map_err(|error| {
            tracing::warn!(%user_id, peer_user_id = %access.peer_user_id, %conversation_id,
                "rejected direct media join by privacy policy");
            social_error(error)
        })?;
    Ok((access, guard))
}

/// Снимает звонки, приглашения и прямое медиаприсутствие пары в обоих направлениях.
pub(crate) async fn terminate_contact_for_block(
    state: &AppState,
    left: &Uuid,
    right: &Uuid,
    conversation_id: Option<Uuid>,
) {
    let removed = if let Some(conversation_id) = conversation_id {
        state
            .voice_presence_store
            .remove_direct_conversation(&conversation_id)
            .await
    } else {
        Vec::new()
    };
    let ended_calls = state.direct_call_store.end_between_users(left, right).await;
    let cancelled_invites = state
        .voice_room_invite_store
        .cancel_between_users(left, right, Utc::now())
        .await;

    for presence in &removed {
        notify_block_eviction(state, presence).await;
    }
    for call in &ended_calls {
        direct_calls::fanout_blocked_call(state, call).await;
    }
    for invite in &cancelled_invites {
        room_invites::fanout_invite_event(state, invite).await;
    }
    tracing::info!(%left, %right, ?conversation_id,
        ended_calls = ended_calls.len(), cancelled_invites = cancelled_invites.len(),
        removed_presences = removed.len(), "terminated blocked voice contacts");
}

async fn notify_block_eviction(
    state: &AppState,
    presence: &crate::features::voice_chat::infrastructure::VoicePresence,
) {
    if presence.voice_protocol_version < 5 {
        state
            .realtime_hub
            .fanout_to_streams(
                RealtimeModule::VoiceChat,
                &presence.server_id,
                RealtimeKind::VoiceChat(VoiceChatKind::ParticipantsChanged),
                &[presence.realtime_stream_id],
                VoiceRoomSnapshot {
                    server_id: presence.server_id.to_string(),
                    room_id: presence.room_id.to_string(),
                    participants: Vec::new(),
                    self_session_id: None,
                    audio_bitrate_bps: None,
                },
            )
            .await;
    } else {
        state
            .realtime_hub
            .fanout_to_streams(
                RealtimeModule::VoiceChat,
                &presence.server_id,
                RealtimeKind::VoiceChat(VoiceChatKind::VoicePresenceEvicted),
                &[presence.realtime_stream_id],
                VoicePresenceEvicted {
                    session_id: presence.session_id.to_string(),
                    joined_at: presence.joined_at.to_rfc3339(),
                    target_kind: VoiceNetworkTargetKind::DirectMessage,
                    server_id: presence.server_id.to_string(),
                    room_id: presence.room_id.to_string(),
                    reason: VoicePresenceEvictionReason::PrivacyBlock,
                },
            )
            .await;
    }
}
