//! Reliable-события завершения видео и связанного UUID системного аудио.

use cheenhub_contracts::realtime::{
    RealtimeKind, RealtimeModule, StopVoiceVideoStream, VoiceChatKind, VoiceVideoStreamSource,
};
use uuid::Uuid;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Сообщает о локальной остановке демонстрации экрана.
///
/// `audio_track_id` адресует завершённый захват: legacy `None` не останавливает аудио.
///
/// # Errors
/// Возвращает ошибку отправки reliable-события текущей realtime-сессии.
pub(crate) async fn send_screen_stream_stopped(
    realtime: &RealtimeHandle,
    server_id: &str,
    room_id: &str,
    audio_track_id: Option<Uuid>,
) -> Result<(), RealtimeError> {
    send_video_stream_stopped(
        realtime,
        server_id,
        room_id,
        VoiceVideoStreamSource::ScreenShare,
        audio_track_id,
    )
    .await
}

/// Сообщает об остановке камеры, не затрагивая аудиодорожки.
///
/// # Errors
/// Возвращает ошибку отправки reliable-события текущей realtime-сессии.
pub(crate) async fn send_camera_stream_stopped(
    realtime: &RealtimeHandle,
    server_id: &str,
    room_id: &str,
) -> Result<(), RealtimeError> {
    send_video_stream_stopped(
        realtime,
        server_id,
        room_id,
        VoiceVideoStreamSource::Camera,
        None,
    )
    .await
}

async fn send_video_stream_stopped(
    realtime: &RealtimeHandle,
    server_id: &str,
    room_id: &str,
    source: VoiceVideoStreamSource,
    audio_track_id: Option<Uuid>,
) -> Result<(), RealtimeError> {
    realtime
        .send_reliable(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::StopVideoStream),
            StopVoiceVideoStream {
                audio_track_id,
                server_id: server_id.to_owned(),
                room_id: room_id.to_owned(),
                source,
            },
        )
        .await
}
