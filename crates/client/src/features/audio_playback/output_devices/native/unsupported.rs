//! Заглушка перечисления устройств вывода для неподдерживаемых платформ.

use dioxus::prelude::debug;

use super::super::contract::AudioOutputDevicesResult;

/// Возвращает отсутствие поддержки перечисления устройств вывода.
pub(crate) async fn enumerate_audio_output_devices() -> AudioOutputDevicesResult {
    debug!("audio output device enumeration is unavailable on this platform");
    AudioOutputDevicesResult {
        devices: None,
        system_managed: false,
        permission_required: false,
    }
}
