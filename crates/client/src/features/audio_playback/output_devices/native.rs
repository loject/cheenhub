//! Выбор реализации перечисления устройств вывода аудио для конкретной платформы.

#[cfg(all(
    not(target_arch = "wasm32"),
    any(
        feature = "android",
        feature = "windows",
        feature = "linux",
        feature = "macos"
    )
))]
mod implementation {
    use cpal::traits::{DeviceTrait, HostTrait};
    use dioxus::prelude::{debug, warn};

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

    /// Возвращает список устройств вывода через `cpal` на Windows, Linux и macOS.
    pub(crate) async fn enumerate_audio_output_devices() -> AudioOutputDevicesResult {
        let host = cpal::default_host();
        let devices = match host.output_devices() {
            Ok(devices) => devices,
            Err(error) => {
                warn!(
                    error = %error,
                    "failed to enumerate native audio output devices"
                );
                return AudioOutputDevicesResult::NotSupported;
            }
        };

        let default_output_name = host
            .default_output_device()
            .and_then(|device| device.name().ok());
        let mut audio_outputs = Vec::new();
        for device in devices {
            let label = match device.name() {
                Ok(name) => name,
                Err(error) => {
                    debug!(
                        error = %error,
                        "skipped native audio output device without readable name"
                    );
                    continue;
                }
            };
            audio_outputs.push(AudioOutputDevice {
                device_id: label.clone(),
                label,
            });
        }

        debug!(
            device_count = audio_outputs.len(),
            has_default_device = default_output_name.is_some(),
            "enumerated native audio output devices"
        );
        if audio_outputs.is_empty() {
            AudioOutputDevicesResult::NoDevices
        } else {
            AudioOutputDevicesResult::Available(audio_outputs)
        }
    }
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(any(
        feature = "android",
        feature = "windows",
        feature = "linux",
        feature = "macos"
    ))
))]
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

    /// Возвращает заглушку для платформ без поддержки перечисления устройств вывода.
    pub(crate) async fn enumerate_audio_output_devices() -> AudioOutputDevicesResult {
        debug!(
            "audio output device enumeration is unavailable without windows, linux or macos feature"
        );
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
