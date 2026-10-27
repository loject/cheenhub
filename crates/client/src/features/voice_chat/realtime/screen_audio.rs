//! Адаптер системного Opus к существующей медиадатаграмме и generic realtime.

use bytes::Bytes;
use cheenhub_contracts::media::{MediaAudioSource, MediaCodec, MediaDatagram, MediaDatagramKind};
use uuid::Uuid;

use crate::features::realtime::{RealtimeError, RealtimeHandle};
use crate::features::screen_share::EncodedScreenAudioFrame;

/// Отправляет системный звук только пока исходный захват не остановлен.
///
/// # Errors
/// Возвращает ошибку UUID комнаты, кодирования пакета или realtime-отправки.
pub(crate) async fn send_screen_audio_frame(
    realtime: &RealtimeHandle,
    room_id: &str,
    frame: EncodedScreenAudioFrame,
) -> Result<(), RealtimeError> {
    if !frame.can_send() {
        return Ok(());
    }
    let room_id =
        Uuid::parse_str(room_id).map_err(|_| RealtimeError::new("Voice room id is invalid."))?;
    let packet = MediaDatagram {
        kind: MediaDatagramKind::VoiceFrame,
        codec: MediaCodec::Opus,
        flags: 0,
        sequence: frame.sequence,
        timestamp_us: frame.timestamp_us,
        duration_us: frame.duration_us,
        room_id,
        sender_user_id: Uuid::nil(),
        track_id: frame.track_id,
        audio_source: Some(MediaAudioSource::System),
        audio_channels: frame.channels,
        payload: frame.bytes.clone(),
    };
    let bytes = packet
        .encode()
        .map_err(|error| RealtimeError::new(format!("Failed to encode screen audio: {error}")))?;
    if !frame.can_send() {
        return Ok(());
    }
    realtime.send_unreliable_bytes(Bytes::from(bytes)).await
}
