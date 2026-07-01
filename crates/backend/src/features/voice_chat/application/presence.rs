//! Поиск активного голосового присутствия пользователя.

use uuid::Uuid;

use crate::features::voice_chat::infrastructure::{VoicePresence, VoicePresenceTargetKind};
use crate::state::AppState;

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
