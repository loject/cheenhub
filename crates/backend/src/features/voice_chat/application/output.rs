//! Изменение состояния входящего звука и рассылка актуального присутствия.

use cheenhub_contracts::realtime::{SetVoiceOutputState, VoiceRoomSnapshot};
use uuid::Uuid;

use super::{
    VoiceChatApplicationError,
    fanout::{fanout_snapshot, room_snapshot},
    parse_id,
};
use crate::state::AppState;

/// Обновляет выбор входящего звука отправителя и рассылает изменившийся снимок.
///
/// Запрос из прежней комнаты, чужого потока или завершённого присутствия
/// отклоняется. Повторное значение не вызывает новую рассылку.
pub(crate) async fn set_output_state(
    state: &AppState,
    stream_id: Uuid,
    user_id: &Uuid,
    request: SetVoiceOutputState,
) -> Result<VoiceRoomSnapshot, VoiceChatApplicationError> {
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    let Some((target, changed)) = state
        .voice_presence_store
        .set_output_muted(&stream_id, user_id, &room_id, request.output_muted)
        .await
    else {
        tracing::warn!(%stream_id, %user_id, %room_id, "rejected output state outside owned voice presence");
        return Err(VoiceChatApplicationError::Unauthorized(
            "Нет активного голосового подключения к этой комнате.".to_owned(),
        ));
    };

    let snapshot = room_snapshot(state, target).await;
    if changed {
        tracing::info!(
            %stream_id,
            %user_id,
            %room_id,
            output_muted = request.output_muted,
            "updated voice participant output mute choice"
        );
        fanout_snapshot(state, target).await;
    }
    Ok(snapshot)
}
