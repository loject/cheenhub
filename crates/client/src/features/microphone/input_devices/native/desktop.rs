//! Перечисление устройств ввода аудио на desktop-платформах.

use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait};
use dioxus::prelude::{debug, warn};

use super::super::contract::{AudioInputDevice, AudioInputDevicesResult};

/// Возвращает список устройств ввода через `cpal`.
///
/// CPAL перечисляет native-устройства синхронно. На Linux эта операция может
/// обходить большое количество ALSA PCM endpoints и заметно блокироваться,
/// поэтому выполняем её в Tokio blocking pool, не занимая Dioxus UI thread.
pub(crate) async fn enumerate_audio_input_devices() -> AudioInputDevicesResult {
    let started = Instant::now();

    let result = match tokio::task::spawn_blocking(enumerate_audio_input_devices_blocking).await {
        Ok(result) => result,
        Err(error) => {
            warn!(
                error = %error,
                "native microphone input device enumeration worker failed"
            );
            unavailable_input_devices()
        }
    };

    debug!(
        elapsed_ms = %started.elapsed().as_millis(),
        device_count = result.devices.as_ref().map_or(0, Vec::len),
        available = result.devices.is_some(),
        "finished native microphone input device enumeration"
    );

    result
}

fn enumerate_audio_input_devices_blocking() -> AudioInputDevicesResult {
    let host = cpal::default_host();
    let devices = match host.input_devices() {
        Ok(devices) => devices,
        Err(error) => {
            warn!(
                error = %error,
                "failed to enumerate native microphone input devices"
            );
            return unavailable_input_devices();
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
    AudioInputDevicesResult {
        devices: Some(audio_inputs),
        system_managed: false,
        permission_required: false,
        permission_denied: false,
    }
}

fn unavailable_input_devices() -> AudioInputDevicesResult {
    AudioInputDevicesResult {
        devices: None,
        system_managed: false,
        permission_required: false,
        permission_denied: false,
    }
}

fn input_device_id(ordinal: usize, label: &str) -> String {
    format!("cpal-input:{ordinal}:{label}")
}

/// На desktop повторно перечисляет устройства без отдельного runtime-разрешения.
pub(crate) async fn request_microphone_permission() -> AudioInputDevicesResult {
    enumerate_audio_input_devices().await
}
