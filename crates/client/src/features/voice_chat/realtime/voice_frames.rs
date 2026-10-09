//! Адаптер голосовых кадров к generic realtime; политика отзыва принадлежит microphone.

use crate::features::microphone::{EncodedMicrophoneFrame, MicrophoneCodec};
use crate::features::realtime::{RealtimeError, RealtimeHandle};
use bytes::Bytes;
use cheenhub_contracts::media::{MediaCodec, MediaDatagram, MediaDatagramKind};
use uuid::Uuid;

/// Отправляет голосовой кадр, если capture всё ещё разрешает передачу.
///
/// Отозванный кадр отбрасывается без сетевого вызова; разрешение остаётся локальным
/// и не входит в media datagram.
///
/// # Errors
/// Возвращает ошибку при невалидном room_id, кодировании или отправке datagram.
pub(crate) async fn send_voice_frame(
    realtime: &RealtimeHandle,
    _server_id: &str,
    room_id: &str,
    frame: EncodedMicrophoneFrame,
) -> Result<(), RealtimeError> {
    if !frame.can_send() {
        return Ok(());
    }
    let permission = frame.permission.clone();
    let room_id =
        Uuid::parse_str(room_id).map_err(|_| RealtimeError::new("Voice room id is invalid."))?;
    let codec = match frame.codec {
        MicrophoneCodec::Opus => MediaCodec::Opus,
    };
    let datagram = MediaDatagram {
        kind: MediaDatagramKind::VoiceFrame,
        codec,
        flags: 0,
        sequence: frame.sequence,
        timestamp_us: frame.timestamp_us,
        duration_us: frame.duration_us,
        room_id,
        sender_user_id: Uuid::nil(),
        payload: frame.bytes,
    };
    let bytes = datagram
        .encode()
        .map_err(|error| RealtimeError::new(format!("Failed to encode voice frame: {error}")))?;

    if permission
        .as_ref()
        .is_some_and(|permission| !permission.allowed())
    {
        return Ok(());
    }
    realtime.send_unreliable_bytes(Bytes::from(bytes)).await
}
