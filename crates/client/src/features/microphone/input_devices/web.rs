//! Перечисление устройств ввода аудио в браузере.

use js_sys::{Array, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{MediaStream, MediaStreamConstraints, window};

use super::super::contract::{AudioInputDevice, AudioInputDevicesResult};
use crate::features::microphone::browser_errors::is_permission_denied_error;

/// Calls `navigator.mediaDevices.enumerateDevices()` and returns the audio inputs.
pub(crate) async fn enumerate_audio_input_devices() -> AudioInputDevicesResult {
    let Some(window) = window() else {
        return unavailable_input_devices();
    };
    let Ok(media_devices) = window.navigator().media_devices() else {
        return unavailable_input_devices();
    };
    let Ok(promise) = media_devices.enumerate_devices() else {
        return unavailable_input_devices();
    };
    let Ok(result) = JsFuture::from(promise).await else {
        return unavailable_input_devices();
    };

    let array = Array::from(&result);
    let mut audio_inputs: Vec<AudioInputDevice> = Vec::new();

    for i in 0..array.length() {
        let item = array.get(i);
        let kind = Reflect::get(&item, &JsValue::from_str("kind"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        if kind != "audioinput" {
            continue;
        }
        let device_id = Reflect::get(&item, &JsValue::from_str("deviceId"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        let label = Reflect::get(&item, &JsValue::from_str("label"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        audio_inputs.push(AudioInputDevice { device_id, label });
    }

    if audio_inputs.is_empty() {
        return available_input_devices(audio_inputs, false);
    }

    let has_labels = audio_inputs.iter().any(|d| !d.label.is_empty());
    if !has_labels {
        return available_input_devices(audio_inputs, true);
    }

    available_input_devices(audio_inputs, false)
}

/// Requests microphone permission, stops the temporary stream, then re-enumerates devices.
pub(crate) async fn request_microphone_permission() -> AudioInputDevicesResult {
    let Some(window) = window() else {
        return unavailable_input_devices();
    };
    let Ok(media_devices) = window.navigator().media_devices() else {
        return unavailable_input_devices();
    };

    let constraints = MediaStreamConstraints::new();
    constraints.set_audio(&JsValue::TRUE);
    constraints.set_video(&JsValue::FALSE);

    let Ok(promise) = media_devices.get_user_media_with_constraints(&constraints) else {
        return unavailable_input_devices();
    };

    match JsFuture::from(promise).await {
        Ok(stream_value) => {
            if let Ok(stream) = stream_value.dyn_into::<MediaStream>() {
                let tracks = stream.get_audio_tracks();
                for i in 0..tracks.length() {
                    if let Ok(track) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>() {
                        track.stop();
                    }
                }
            }
            enumerate_audio_input_devices().await
        }
        Err(error) => {
            if is_permission_denied_error(&error) {
                AudioInputDevicesResult {
                    devices: None,
                    system_managed: false,
                    permission_required: false,
                    permission_denied: true,
                }
            } else {
                unavailable_input_devices()
            }
        }
    }
}

fn available_input_devices(
    devices: Vec<AudioInputDevice>,
    permission_required: bool,
) -> AudioInputDevicesResult {
    AudioInputDevicesResult {
        devices: Some(devices),
        system_managed: false,
        permission_required,
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
