//! Изменение явного выбора микрофона и рассылка актуального присутствия.

use cheenhub_contracts::realtime::{SetVoiceMicrophoneState, VoiceRoomSnapshot};
use uuid::Uuid;

use super::{
    VoiceChatApplicationError,
    fanout::{fanout_snapshot, room_snapshot},
    parse_id,
};
use crate::state::AppState;

/// Меняет микрофон присутствия отправителя и рассылает изменившийся снимок.
///
/// UUID комнаты защищает от запросов предыдущей цели на том же realtime-потоке.
/// Повторный запрос идемпотентен; чужое или завершённое присутствие отклоняется.
pub(crate) async fn set_microphone_state(
    state: &AppState,
    stream_id: Uuid,
    user_id: &Uuid,
    request: SetVoiceMicrophoneState,
) -> Result<VoiceRoomSnapshot, VoiceChatApplicationError> {
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    let Some((target, changed)) = state
        .voice_presence_store
        .set_microphone_enabled(&stream_id, user_id, &room_id, request.microphone_enabled)
        .await
    else {
        tracing::warn!(%stream_id, %user_id, %room_id, "rejected microphone state outside owned voice presence");
        return Err(VoiceChatApplicationError::Unauthorized(
            "Нет активного голосового подключения к этой комнате.".to_owned(),
        ));
    };
    let snapshot = room_snapshot(state, target).await;
    if changed {
        tracing::info!(%stream_id, %user_id, %room_id, microphone_enabled = request.microphone_enabled,
            "updated voice participant microphone choice");
        fanout_snapshot(state, target).await;
    }
    Ok(snapshot)
}
