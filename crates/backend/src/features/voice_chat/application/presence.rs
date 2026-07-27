//! Поиск активного голосового присутствия пользователя.

use uuid::Uuid;

use crate::features::voice_chat::infrastructure::{VoicePresence, VoicePresenceTargetKind};
use crate::state::AppState;

use super::{direct_calls, fanout::fanout_removed_rooms};

pub(super) async fn active_presence_for_user(
    state: &AppState,
    room_id: &Uuid,
    user_id: &Uuid,
) -> Option<VoicePresence> {
    if let Some(presence) = state
        .voice_presence_store
        .room_presence_for_user(VoicePresenceTargetKind::Server, room_id, user_id)
        .await
    {
        return Some(presence);
    }
    state
        .voice_presence_store
        .room_presence_for_user(VoicePresenceTargetKind::DirectMessage, room_id, user_id)
        .await
}

/// Удаляет присутствие закрытого realtime-потока и завершает связанный личный звонок.
pub(crate) async fn disconnect_realtime_stream(state: &AppState, realtime_stream_id: Uuid) {
    let removed = state
        .voice_presence_store
        .leave_realtime_stream(&realtime_stream_id)
        .await;
    let direct_message_calls = removed
        .iter()
        .filter(|presence| presence.target_kind == VoicePresenceTargetKind::DirectMessage)
        .map(|presence| (presence.user_id, presence.room_id))
        .collect::<Vec<_>>();
    fanout_removed_rooms(state, removed, None).await;
    for (user_id, conversation_id) in direct_message_calls {
        direct_calls::end_direct_call_for_presence(state, &user_id, &conversation_id).await;
    }
}
