//! Выбор платформенной реализации перечисления устройств ввода аудио.

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
    #[cfg(feature = "android")]
    use crate::features::runtime::android::{AndroidPermission, PermissionResult, android_bridge};
    use cpal::traits::{DeviceTrait, HostTrait};
    use dioxus::prelude::{debug, warn};

    use crate::features::microphone::native::device_key::input_device_id;

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

    /// Возвращает список устройств ввода через `cpal` на Windows, Linux и macOS.
    pub(crate) async fn enumerate_audio_input_devices() -> AudioInputDevicesResult {
        let host = cpal::default_host();
        let devices = match host.input_devices() {
            Ok(devices) => devices,
            Err(error) => {
                warn!(
                    error = %error,
                    "failed to enumerate native microphone input devices"
                );
                return AudioInputDevicesResult::NotSupported;
            }
        };

        let default_input_name = host
            .default_input_device()
            .and_then(|device| device.name().ok());
        let mut audio_inputs = Vec::new();
        for (ordinal, device) in devices.enumerate() {
            let label = match device.name() {
                Ok(name) => name,
                Err(error) => {
                    debug!(
                        error = %error,
                        "skipped native microphone input device without readable name"
                    );
                    continue;
                }
            };
            audio_inputs.push(AudioInputDevice {
                device_id: input_device_id(ordinal, &label),
                label,
            });
        }

        debug!(
            device_count = audio_inputs.len(),
            has_default_device = default_input_name.is_some(),
            "enumerated native microphone input devices"
        );
        if audio_inputs.is_empty() {
            AudioInputDevicesResult::NoDevices
        } else {
            AudioInputDevicesResult::Available(audio_inputs)
        }
    }

    /// Запрашивает runtime-разрешение Android или сразу перечисляет устройства на desktop.
    pub(crate) async fn request_microphone_permission() -> AudioInputDevicesResult {
        #[cfg(feature = "android")]
        {
            let bridge = match android_bridge() {
                Ok(bridge) => bridge,
                Err(error) => {
                    warn!(error = %error, "Android microphone permission bridge is unavailable");
                    return AudioInputDevicesResult::NotSupported;
                }
            };
            let (sender, receiver) = futures_channel::oneshot::channel();
            if let Err(error) = bridge.request_permission(
                AndroidPermission::RecordAudio,
                Box::new(move |result| {
                    let _ = sender.send(result);
                }),
            ) {
                warn!(error = %error, "failed to request Android microphone permission");
                return AudioInputDevicesResult::NotSupported;
            }

            return match receiver.await {
                Ok(Ok(PermissionResult::Granted)) => enumerate_audio_input_devices().await,
                Ok(Ok(PermissionResult::Denied | PermissionResult::DeniedPermanently)) => {
                    AudioInputDevicesResult::PermissionDenied
                }
                Ok(Err(error)) => {
                    warn!(error = %error, "Android microphone permission request failed");
                    AudioInputDevicesResult::NotSupported
                }
                Err(error) => {
                    warn!(error = %error, "Android microphone permission callback was dropped");
                    AudioInputDevicesResult::NotSupported
                }
            };
        }

        #[cfg(not(feature = "android"))]
        enumerate_audio_input_devices().await
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

    /// Возвращает заглушку для host-сборок без platform feature.
    pub(crate) async fn enumerate_audio_input_devices() -> AudioInputDevicesResult {
        debug!(
            "microphone input device enumeration is unavailable without windows, linux or macos feature"
        );
        AudioInputDevicesResult::NotSupported
    }

    /// Возвращает заглушку запроса разрешения для host-сборок без platform feature.
    pub(crate) async fn request_microphone_permission() -> AudioInputDevicesResult {
        debug!(
            "microphone permission request is unavailable without windows, linux or macos feature"
        );
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
