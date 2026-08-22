//! Заглушка перечисления устройств ввода для неподдерживаемых платформ.

use dioxus::prelude::debug;

use super::super::contract::AudioInputDevicesResult;

/// Возвращает отсутствие поддержки перечисления устройств ввода.
pub(crate) async fn enumerate_audio_input_devices() -> AudioInputDevicesResult {
    debug!("microphone input device enumeration is unavailable on this platform");
    unavailable_input_devices()
}

/// Возвращает отсутствие поддержки запроса разрешения.
pub(crate) async fn request_microphone_permission() -> AudioInputDevicesResult {
    debug!("microphone permission request is unavailable on this platform");
    unavailable_input_devices()
}

fn unavailable_input_devices() -> AudioInputDevicesResult {
    AudioInputDevicesResult {
        devices: None,
        system_managed: false,
        permission_required: false,
        permission_denied: false,
    }
}
