//! Запуск микрофона через подходящий для платформы uplink.

use crate::features::microphone::MicrophoneHandle;
use crate::features::realtime::RealtimeHandle;
use dioxus::prelude::info;

use super::state::VoiceConnectionHandle;

/// Восстанавливает захват только при разрешённом входящем звуке и выборе пользователя.
pub(crate) fn restart(
    microphone: MicrophoneHandle,
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
    voice: VoiceConnectionHandle,
) {
    if !microphone.voice_enabled()
        || voice.output_muted()
        || voice
            .outbound_session_id_for(&server_id, &room_id)
            .is_none()
    {
        microphone.stop();
        return;
    }
    super::microphone_uplink_platform::restart(microphone, realtime, server_id, room_id, voice);
}

/// Меняет выбор микрофона; попытка включения при deafen сначала снимает deafen.
pub(crate) fn toggle(
    microphone: MicrophoneHandle,
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
    voice: Option<VoiceConnectionHandle>,
) {
    if voice
        .as_ref()
        .is_some_and(VoiceConnectionHandle::output_muted)
    {
        microphone.set_voice_enabled(true);
        if let Some(voice) = &voice {
            voice.set_output_muted(false);
        }
        info!(%server_id, %room_id, "enabled incoming voice audio and microphone by user request");
        return;
    }

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
    info!(%server_id, %room_id, microphone_enabled = enabled, "applied user microphone toggle");
}
