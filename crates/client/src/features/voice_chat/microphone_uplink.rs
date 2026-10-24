//! Запуск микрофона через подходящий для платформы uplink.

use crate::features::microphone::MicrophoneHandle;
use crate::features::realtime::RealtimeHandle;
use dioxus::prelude::info;

/// Восстанавливает захват только при сохранённом разрешении пользователя.
pub(crate) fn restart(
    microphone: MicrophoneHandle,
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
) {
    if !microphone.voice_enabled() {
        microphone.stop();
        return;
    }
    super::microphone_uplink_platform::restart(microphone, realtime, server_id, room_id);
}

/// Меняет сохраняемый выбор; ошибки и временные остановки его не сбрасывают.
pub(crate) fn toggle(
    microphone: MicrophoneHandle,
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
) {
    let enabled = !microphone.voice_enabled();
    microphone.set_voice_enabled(enabled);
    if enabled {
        restart(microphone, realtime, server_id.clone(), room_id.clone());
    } else {
        microphone.stop();
    }
    info!(%server_id, %room_id, "applied user microphone toggle");
}
