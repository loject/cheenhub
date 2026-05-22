//! Browser microphone stream acquisition.

use dioxus::prelude::{debug, warn};
use js_sys::{Function, Object, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{MediaStream, MediaStreamConstraints, MediaTrackConstraints, window};

use super::backend::{MicrophoneConfig, MicrophoneError};
use super::browser_errors::{
    is_device_constraint_error, is_permission_denied_error, js_error_message,
};

pub(super) async fn request_microphone_stream(
    config: MicrophoneConfig,
) -> Result<MediaStream, MicrophoneError> {
    let requested_device_id = config.device_id.clone();
    let window = window().ok_or_else(|| MicrophoneError::new("Окно браузера недоступно."))?;
    let media_devices = window
        .navigator()
        .media_devices()
        .map_err(|error| MicrophoneError::new(js_error_message(error)))?;
    let audio = microphone_track_constraints(&config);

    let constraints = MediaStreamConstraints::new();
    constraints.set_audio(&audio);
    constraints.set_video(&JsValue::FALSE);

    let promise = media_devices
        .get_user_media_with_constraints(&constraints)
        .map_err(microphone_error)?;
    let stream = match JsFuture::from(promise).await {
        Ok(stream) => stream,
        Err(error) if requested_device_id.is_some() && is_device_constraint_error(&error) => {
            warn!(
                requested_device = requested_device_id.as_deref().map(device_log_key),
                "preferred microphone input device unavailable; falling back to default input"
            );
            return request_default_microphone_stream(&media_devices, config).await;
        }
        Err(error) => return Err(microphone_error(error)),
    };

    stream
        .dyn_into::<MediaStream>()
        .map_err(|_| MicrophoneError::new("Браузер вернул некорректный поток микрофона."))
}

pub(super) fn first_audio_track(
    stream: &MediaStream,
) -> Result<web_sys::MediaStreamTrack, MicrophoneError> {
    let tracks = stream.get_audio_tracks();
    let Some(track) = tracks.get(0).dyn_into::<web_sys::MediaStreamTrack>().ok() else {
        return Err(MicrophoneError::new("Микрофон не вернул аудиодорожку."));
    };

    Ok(track)
}

pub(super) fn log_selected_audio_track(
    track: &web_sys::MediaStreamTrack,
    requested_device_id: Option<&str>,
) {
    let actual_device_id = track_settings_device_id(track);
    debug!(
        requested_device = requested_device_id.map(device_log_key),
        actual_device = actual_device_id.as_deref().map(device_log_key),
        "browser microphone capture track selected"
    );
    if requested_device_id.is_some()
        && actual_device_id.is_some()
        && requested_device_id != actual_device_id.as_deref()
    {
        warn!(
            requested_device = requested_device_id.map(device_log_key),
            actual_device = actual_device_id.as_deref().map(device_log_key),
            "browser returned microphone track for a different input device"
        );
    }
}

async fn request_default_microphone_stream(
    media_devices: &web_sys::MediaDevices,
    mut config: MicrophoneConfig,
) -> Result<MediaStream, MicrophoneError> {
    config.device_id = None;
    let audio = microphone_track_constraints(&config);

    let constraints = MediaStreamConstraints::new();
    constraints.set_audio(&audio);
    constraints.set_video(&JsValue::FALSE);

    let promise = media_devices
        .get_user_media_with_constraints(&constraints)
        .map_err(microphone_error)?;
    let stream = JsFuture::from(promise).await.map_err(microphone_error)?;

    stream
        .dyn_into::<MediaStream>()
        .map_err(|_| MicrophoneError::new("Браузер вернул некорректный поток микрофона."))
}

fn microphone_track_constraints(config: &MicrophoneConfig) -> MediaTrackConstraints {
    let audio = MediaTrackConstraints::new();
    audio.set_channel_count(&JsValue::from_f64(f64::from(config.channels)));
    audio.set_echo_cancellation(&JsValue::TRUE);
    audio.set_noise_suppression(&JsValue::TRUE);
    audio.set_auto_gain_control(&JsValue::TRUE);
    if let Some(ref device_id) = config.device_id
        && !device_id.is_empty()
    {
        audio.set_device_id(&exact_device_id_constraint(device_id));
    }
    audio
}

fn exact_device_id_constraint(device_id: &str) -> JsValue {
    let exact = Object::new();
    set_property(&exact, "exact", &JsValue::from_str(device_id));
    exact.into()
}

fn track_settings_device_id(track: &web_sys::MediaStreamTrack) -> Option<String> {
    let get_settings = Reflect::get(track.as_ref(), &JsValue::from_str("getSettings"))
        .ok()?
        .dyn_into::<Function>()
        .ok()?;
    let settings = get_settings.call0(track.as_ref()).ok()?;
    Reflect::get(&settings, &JsValue::from_str("deviceId"))
        .ok()
        .and_then(|value| value.as_string())
        .filter(|value| !value.is_empty())
}

fn device_log_key(device_id: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in device_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn microphone_error(error: JsValue) -> MicrophoneError {
    if is_permission_denied_error(&error) {
        MicrophoneError::permission_denied("Доступ к микрофону запрещен.")
    } else {
        MicrophoneError::new(js_error_message(error))
    }
}

fn set_property(object: &Object, name: &str, value: &JsValue) {
    let _ = Reflect::set(object, &JsValue::from_str(name), value);
}
