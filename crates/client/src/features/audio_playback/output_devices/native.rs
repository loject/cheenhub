//! Выбор платформенной реализации перечисления устройств вывода аудио.

#[cfg(not(target_arch = "wasm32"))]
mod implementation {
    use dioxus::prelude::debug;

    /// Одно устройство вывода аудио.
    #[derive(Debug, Clone, PartialEq)]
    pub(crate) struct AudioOutputDevice {
        /// Идентификатор устройства вывода.
        pub(crate) device_id: String,
        /// Отображаемое имя устройства вывода.
        pub(crate) label: String,
    }

    /// Результат перечисления устройств вывода аудио.
    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq)]
    pub(crate) enum AudioOutputDevicesResult {
        /// API перечисления устройств недоступен.
        NotSupported,
        /// Устройства есть, но подписи скрыты до выдачи разрешения.
        PermissionRequired,
        /// Устройства вывода аудио не найдены.
        NoDevices,
        /// Доступен список устройств вывода аудио.
        Available(Vec<AudioOutputDevice>),
    }

    /// Возвращает native-заглушку перечисления устройств вывода.
    pub(crate) async fn enumerate_audio_output_devices() -> AudioOutputDevicesResult {
        debug!("audio output device enumeration is unavailable on native client");
        AudioOutputDevicesResult::NotSupported
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) use super::web::{
    AudioOutputDevice, AudioOutputDevicesResult, enumerate_audio_output_devices,
};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use implementation::{
    AudioOutputDevice, AudioOutputDevicesResult, enumerate_audio_output_devices,
};
