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

/// Завершает голосовое присутствие в комнатах удалённого сервера.
///
/// Уведомляет только потоки удалённых участников: после удаления membership
/// обычная серверная фильтрация уже не найдёт получателей. Остальные realtime
/// соединения пользователей продолжают работать.
pub(crate) async fn remove_deleted_server(state: &AppState, server_id: Uuid) {
    use cheenhub_contracts::realtime::{
        RealtimeKind, RealtimeModule, VoiceChatKind, VoiceRoomSnapshot,
    };

    let removed = state.voice_presence_store.remove_server(&server_id).await;
    tracing::info!(%server_id, removed_presences = removed.len(), "removed voice presence for deleted server");
    for presence in removed {
        state
            .realtime_hub
            .fanout_to_streams(
                RealtimeModule::VoiceChat,
                &server_id,
                RealtimeKind::VoiceChat(VoiceChatKind::ParticipantsChanged),
                &[presence.realtime_stream_id],
                VoiceRoomSnapshot {
                    server_id: server_id.to_string(),
                    room_id: presence.room_id.to_string(),
                    participants: Vec::new(),
                    audio_bitrate_bps: None,
                },
            )
            .await;
    }
}

/// Завершает замену присутствия и проверяет доступность сервера после записи.
///
/// Снимок покинутой комнаты рассылается даже при отказе новой цели. Ошибка
/// хранилища не должна оставлять присутствие входа, который клиенту отклонён.
pub(super) async fn finish_server_join(
    state: &AppState,
    stream_id: Uuid,
    target: crate::features::voice_chat::infrastructure::VoicePresenceTarget,
    replaced: Vec<VoicePresence>,
    available: anyhow::Result<bool>,
) -> Result<(), super::VoiceChatApplicationError> {
    fanout_removed_rooms(state, replaced, Some(target)).await;
    match available {
        Ok(true) => Ok(()),
        Ok(false) => {
            remove_deleted_server(state, target.server_id).await;
            tracing::warn!(server_id = %target.server_id, room_id = %target.room_id, "voice join raced with server deletion");
            Err(super::VoiceChatApplicationError::NotFound(
                "Сервер не найден.".to_owned(),
            ))
        }
        Err(error) => {
            let removed = state
                .voice_presence_store
                .leave_room(&stream_id, target.kind, &target.server_id, &target.room_id)
                .await;
            fanout_removed_rooms(state, removed, None).await;
            tracing::warn!(server_id = %target.server_id, room_id = %target.room_id, %error, "failed to verify joined voice server; removed rejected presence");
            Err(super::VoiceChatApplicationError::Internal(error))
        }
    }
}
