//! Browser audio output device discovery.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use js_sys::{Array, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::window;

/// One audio output device returned by `enumerateDevices`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AudioOutputDevice {
    pub(crate) device_id: String,
    pub(crate) label: String,
}

/// Result of enumerating the browser's audio output devices.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AudioOutputDevicesResult {
    /// The browser does not expose the MediaDevices API.
    NotSupported,
    /// Devices are listed but have no labels because media permission is not granted.
    PermissionRequired,
    /// No audio output devices are exposed by the browser.
    NoDevices,
    /// At least one labeled audio output device is available.
    Available(Vec<AudioOutputDevice>),
}

/// Calls `navigator.mediaDevices.enumerateDevices()` and returns audio outputs.
pub(crate) async fn enumerate_audio_output_devices() -> AudioOutputDevicesResult {
    let Some(window) = window() else {
        return AudioOutputDevicesResult::NotSupported;
    };
    let Ok(media_devices) = window.navigator().media_devices() else {
        return AudioOutputDevicesResult::NotSupported;
    };
    let Ok(promise) = media_devices.enumerate_devices() else {
        return AudioOutputDevicesResult::NotSupported;
    };
    let Ok(result) = JsFuture::from(promise).await else {
        return AudioOutputDevicesResult::NotSupported;
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
        return AudioOutputDevicesResult::NoDevices;
    }

    let has_labels = audio_outputs.iter().any(|device| !device.label.is_empty());
    if !has_labels {
        return AudioOutputDevicesResult::PermissionRequired;
    }

    AudioOutputDevicesResult::Available(audio_outputs)
}
