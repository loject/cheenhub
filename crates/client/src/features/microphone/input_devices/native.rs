//! Выбор платформенной реализации перечисления устройств ввода аудио.

#[cfg(not(target_arch = "wasm32"))]
mod implementation {
    use dioxus::prelude::debug;

    /// Одно устройство ввода аудио.
    #[derive(Debug, Clone, PartialEq)]
    pub(crate) struct AudioInputDevice {
        /// Идентификатор устройства ввода.
        pub(crate) device_id: String,
        /// Отображаемое имя устройства ввода.
        pub(crate) label: String,
    }

    /// Результат перечисления устройств ввода аудио.
    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq)]
    pub(crate) enum AudioInputDevicesResult {
        /// API перечисления устройств недоступен.
        NotSupported,
        /// Устройства есть, но подписи скрыты до выдачи разрешения.
        PermissionRequired,
        /// Пользователь запретил доступ к микрофону.
        PermissionDenied,
        /// Устройства ввода аудио не найдены.
        NoDevices,
        /// Доступен список устройств ввода аудио.
        Available(Vec<AudioInputDevice>),
    }

    /// Возвращает native-заглушку перечисления устройств ввода.
    pub(crate) async fn enumerate_audio_input_devices() -> AudioInputDevicesResult {
        debug!("microphone input device enumeration is unavailable on native client");
        AudioInputDevicesResult::NotSupported
    }

    /// Возвращает native-заглушку запроса разрешения микрофона.
    pub(crate) async fn request_microphone_permission() -> AudioInputDevicesResult {
        debug!("microphone permission request is unavailable on native client");
        AudioInputDevicesResult::NotSupported
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) use super::web::{
    AudioInputDevice, AudioInputDevicesResult, enumerate_audio_input_devices,
    request_microphone_permission,
};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use implementation::{
    AudioInputDevice, AudioInputDevicesResult, enumerate_audio_input_devices,
    request_microphone_permission,
};
