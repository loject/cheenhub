//! Перечисление устройств вывода аудио на Android.

use dioxus::prelude::info;

use super::super::contract::AudioOutputDevicesResult;

/// Сообщает, что маршрут вывода управляется Android `AudioManager`.
pub(crate) async fn enumerate_audio_output_devices() -> AudioOutputDevicesResult {
    info!(
        platform = "android",
        management = "audio_manager",
        "audio output device selection is managed by the system"
    );
    AudioOutputDevicesResult::SystemManaged
}
