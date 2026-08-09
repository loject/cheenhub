//! Перечисление устройств ввода аудио на desktop-платформах.

use cpal::traits::{DeviceTrait, HostTrait};
use dioxus::prelude::{debug, warn};

use super::super::contract::{AudioInputDevice, AudioInputDevicesResult};
use crate::features::microphone::native::device_key::input_device_id;

/// Возвращает список устройств ввода через `cpal`.
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

/// На desktop повторно перечисляет устройства без отдельного runtime-разрешения.
pub(crate) async fn request_microphone_permission() -> AudioInputDevicesResult {
    enumerate_audio_input_devices().await
}
