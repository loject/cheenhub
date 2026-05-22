//! Browser audio input device discovery.

use js_sys::{Array, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{MediaStream, MediaStreamConstraints, window};

use super::browser_errors::is_permission_denied_error;

/// One audio input device returned by `enumerateDevices`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AudioInputDevice {
    pub(crate) device_id: String,
    pub(crate) label: String,
}

/// Result of enumerating the browser's audio input devices.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AudioInputDevicesResult {
    /// The browser does not expose the MediaDevices API.
    NotSupported,
    /// Devices are listed but have no labels because microphone permission is not granted.
    PermissionRequired,
    /// The user explicitly denied microphone access.
    PermissionDenied,
    /// No audio input devices are connected.
    NoDevices,
    /// At least one labeled audio input device is available.
    Available(Vec<AudioInputDevice>),
}

/// Calls `navigator.mediaDevices.enumerateDevices()` and returns the audio inputs.
pub(crate) async fn enumerate_audio_input_devices() -> AudioInputDevicesResult {
    let Some(window) = window() else {
        return AudioInputDevicesResult::NotSupported;
    };
    let Ok(media_devices) = window.navigator().media_devices() else {
        return AudioInputDevicesResult::NotSupported;
    };
    let Ok(promise) = media_devices.enumerate_devices() else {
        return AudioInputDevicesResult::NotSupported;
    };
    let Ok(result) = JsFuture::from(promise).await else {
        return AudioInputDevicesResult::NotSupported;
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
        return AudioInputDevicesResult::NoDevices;
    }

    let has_labels = audio_inputs.iter().any(|d| !d.label.is_empty());
    if !has_labels {
        return AudioInputDevicesResult::PermissionRequired;
    }

    AudioInputDevicesResult::Available(audio_inputs)
}

/// Requests microphone permission, stops the temporary stream, then re-enumerates devices.
pub(crate) async fn request_microphone_permission() -> AudioInputDevicesResult {
    let Some(window) = window() else {
        return AudioInputDevicesResult::NotSupported;
    };
    let Ok(media_devices) = window.navigator().media_devices() else {
        return AudioInputDevicesResult::NotSupported;
    };

    let constraints = MediaStreamConstraints::new();
    constraints.set_audio(&JsValue::TRUE);
    constraints.set_video(&JsValue::FALSE);

    let Ok(promise) = media_devices.get_user_media_with_constraints(&constraints) else {
        return AudioInputDevicesResult::NotSupported;
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
                AudioInputDevicesResult::PermissionDenied
            } else {
                AudioInputDevicesResult::NotSupported
            }
        }
    }
}
