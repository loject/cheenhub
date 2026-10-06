//! Остановка исходящего видеопотока голосовой комнаты.

use cheenhub_contracts::media::MediaDatagramKind;
use cheenhub_contracts::realtime::{
    RealtimeKind, RealtimeModule, StopVoiceVideoStream, VoiceChatKind, VoiceVideoStreamEnded,
    VoiceVideoStreamSource,
};
use uuid::Uuid;

use crate::state::AppState;

use super::VoiceChatApplicationError;
use super::parse_id;
use super::presence::active_presence_for_user;

/// Рассылает участникам комнаты событие остановки видеопотока отправителя.
pub(crate) async fn stop_video_stream(
    state: &AppState,
    realtime_stream_id: Uuid,
    session_id: Uuid,
    user_id: &Uuid,
    request: StopVoiceVideoStream,
) -> Result<(), VoiceChatApplicationError> {
    let server_id = parse_id(&request.server_id, "Сервер не найден.")?;
    let room_id = parse_id(&request.room_id, "Комната не найдена.")?;
    let Some(presence) = active_presence_for_user(state, &room_id, user_id) else {
        return Err(VoiceChatApplicationError::NotFound(
            "Пользователь не находится в этой голосовой комнате.".to_owned(),
        ));
    };

    if presence.server_id != server_id || presence.room_id != room_id {
        return Err(VoiceChatApplicationError::BadRequest(
            "Комната не найдена.".to_owned(),
        ));
    }
    if presence.realtime_stream_id != realtime_stream_id || presence.session_id != session_id {
        return Err(VoiceChatApplicationError::Unauthorized(
            "Видеопоток принадлежит другой realtime-сессии.".to_owned(),
        ));
    }

    state
        .voice_presence_store
        .remove_video_source(session_id, room_id, media_kind_for_source(request.source))
        .await;
    tracing::debug!(
        %server_id,
        %room_id,
        %session_id,
        %user_id,
        source = ?request.source,
        "removed stopped voice video source from host activity"
    );

    let recipients = state
        .voice_presence_store
        .room_participants(presence.target_kind, &server_id, &room_id)
        .await;
    let stream_ids = recipients
        .iter()
        .filter(|recipient| recipient.realtime_stream_id != realtime_stream_id)
        .map(|recipient| recipient.realtime_stream_id)
        .collect::<Vec<_>>();
    tracing::info!(
        server_id = %server_id,
        room_id = %room_id,
        target_kind = ?presence.target_kind,
        user_id = %user_id,
        source = ?request.source,
        recipients = stream_ids.len(),
        "fanning out voice video stream ended event"
    );

    state
        .realtime_hub
        .fanout_to_streams(
            RealtimeModule::VoiceChat,
            &server_id,
            RealtimeKind::VoiceChat(VoiceChatKind::VideoStreamEnded),
            &stream_ids,
            VoiceVideoStreamEnded {
                server_id: server_id.to_string(),
                room_id: room_id.to_string(),
                user_id: user_id.to_string(),
                source: request.source,
            },
        )
        .await;

    Ok(())
}
fn media_kind_for_source(source: VoiceVideoStreamSource) -> MediaDatagramKind {
    match source {
        VoiceVideoStreamSource::Camera => MediaDatagramKind::CameraFrame,
        VoiceVideoStreamSource::ScreenShare => MediaDatagramKind::ScreenFrame,
    }
}
