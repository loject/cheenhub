//! Запуск микрофона через подходящий для платформы uplink.

use crate::features::microphone::MicrophoneHandle;
use crate::features::realtime::RealtimeHandle;
use dioxus::prelude::info;

use super::state::VoiceConnectionHandle;

/// Восстанавливает захват только при сохранённом разрешении пользователя.
pub(crate) fn restart(
    microphone: MicrophoneHandle,
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
    voice: VoiceConnectionHandle,
) {
    if !microphone.voice_enabled()
        || voice
            .outbound_session_id_for(&server_id, &room_id)
            .is_none()
    {
        microphone.stop();
        return;
    }
    super::microphone_uplink_platform::restart(microphone, realtime, server_id, room_id, voice);
}

/// Меняет сохраняемый выбор; ошибки и временные остановки его не сбрасывают.
pub(crate) fn toggle(
    microphone: MicrophoneHandle,
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
    voice: Option<VoiceConnectionHandle>,
) {
    let enabled = !microphone.voice_enabled();
    microphone.set_voice_enabled(enabled);
    if enabled {
        if let Some(voice) = voice {
            restart(
                microphone,
                realtime,
                server_id.clone(),
                room_id.clone(),
                voice,
            );
        } else {
            microphone.stop();
        }
    } else {
        microphone.stop();
    }
    info!(%server_id, %room_id, "applied user microphone toggle");
}
