//! Перечисление устройств вывода аудио на desktop-платформах.

use cpal::traits::{DeviceTrait, HostTrait};
use dioxus::prelude::{debug, warn};

use super::super::contract::{AudioOutputDevice, AudioOutputDevicesResult};

/// Возвращает список устройств вывода через `cpal`.
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
