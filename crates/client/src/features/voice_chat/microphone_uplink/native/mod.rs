//! Отправка микрофона через основную realtime-сессию на native-платформах.

use crate::features::microphone::MicrophoneHandle;
use crate::features::realtime::RealtimeHandle;

use super::super::state::VoiceConnectionHandle;
use super::super::voice_frame_sender;

/// Перезапускает микрофон для выбранной комнаты.
pub(crate) fn restart(
    microphone: MicrophoneHandle,
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
    voice: VoiceConnectionHandle,
) {
    microphone.restart(voice_frame_sender::voice_frame_sender_callback(
        realtime, server_id, room_id, voice,
    ));
}

#[cfg(test)]
mod tests;
