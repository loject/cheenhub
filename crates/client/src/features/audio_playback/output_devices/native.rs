//! Выбор платформенной реализации перечисления устройств вывода аудио.

#[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
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

    /// Возвращает список native-устройств вывода через `cpal`.
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

        if audio_outputs.is_empty() {
            AudioOutputDevicesResult::NoDevices
        } else {
            AudioOutputDevicesResult::Available(audio_outputs)
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "native-audio")))]
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
        debug!("audio output device enumeration is unavailable without native audio feature");
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
