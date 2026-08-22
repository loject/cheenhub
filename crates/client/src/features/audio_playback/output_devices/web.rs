//! Перечисление устройств вывода аудио в браузере.

use js_sys::{Array, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::window;

use super::super::contract::{AudioOutputDevice, AudioOutputDevicesResult};

/// Calls `navigator.mediaDevices.enumerateDevices()` and returns audio outputs.
pub(crate) async fn enumerate_audio_output_devices() -> AudioOutputDevicesResult {
    let Some(window) = window() else {
        return unavailable_output_devices();
    };
    let Ok(media_devices) = window.navigator().media_devices() else {
        return unavailable_output_devices();
    };
    let Ok(promise) = media_devices.enumerate_devices() else {
        return unavailable_output_devices();
    };
    let Ok(result) = JsFuture::from(promise).await else {
        return unavailable_output_devices();
    };

    let array = Array::from(&result);
    let mut audio_outputs = Vec::new();

    for i in 0..array.length() {
        let item = array.get(i);
        let kind = Reflect::get(&item, &JsValue::from_str("kind"))
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_default();
        if kind != "audiooutput" {
            continue;
        }
        let device_id = Reflect::get(&item, &JsValue::from_str("deviceId"))
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_default();
        let label = Reflect::get(&item, &JsValue::from_str("label"))
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_default();
        audio_outputs.push(AudioOutputDevice { device_id, label });
    }

    if audio_outputs.is_empty() {
        return AudioOutputDevicesResult {
            devices: Some(audio_outputs),
            system_managed: false,
            permission_required: false,
        };
    }

    let has_labels = audio_outputs.iter().any(|device| !device.label.is_empty());
    if !has_labels {
        return AudioOutputDevicesResult {
            devices: Some(audio_outputs),
            system_managed: false,
            permission_required: true,
        };
    }

    AudioOutputDevicesResult {
        devices: Some(audio_outputs),
        system_managed: false,
        permission_required: false,
    }
}

fn unavailable_output_devices() -> AudioOutputDevicesResult {
    AudioOutputDevicesResult {
        devices: None,
        system_managed: false,
        permission_required: false,
    }
}
