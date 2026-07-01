//! Browser microphone stream acquisition.

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

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
use super::browser_worklet::browser_media_diagnostics_enabled;

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
    if browser_media_diagnostics_enabled() {
        let settings = track_settings_summary(track);
        debug!(
            requested_device = requested_device_id.map(device_log_key),
            actual_device = actual_device_id.as_deref().map(device_log_key),
            channel_count = settings.channel_count,
            sample_rate = settings.sample_rate,
            echo_cancellation = settings.echo_cancellation,
            noise_suppression = settings.noise_suppression,
            auto_gain_control = settings.auto_gain_control,
            voice_isolation = settings.voice_isolation,
            "browser microphone capture track selected"
        );
    } else {
        debug!(
            requested_device = requested_device_id.map(device_log_key),
            actual_device = actual_device_id.as_deref().map(device_log_key),
            "browser microphone capture track selected"
        );
    }
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
    let browser_voice_processing = JsValue::from_bool(browser_voice_processing_enabled());
    audio.set_echo_cancellation(&browser_voice_processing);
    audio.set_noise_suppression(&browser_voice_processing);
    audio.set_auto_gain_control(&browser_voice_processing);
    // Усиленная ML-изоляция голоса Chrome (deep noise suppression) поверх обычного
    // noiseSuppression: лучше давит постоянный фоновый шум (вентилятор, улица, набор
    // на клавиатуре). Это нестандартный constraint, поэтому задаем его через Reflect;
    // движки без поддержки просто игнорируют неизвестный ключ.
    let _ = Reflect::set(
        audio.as_ref(),
        &JsValue::from_str("voiceIsolation"),
        &browser_voice_processing,
    );
    if let Some(ref device_id) = config.device_id
        && !device_id.is_empty()
    {
        audio.set_device_id(&exact_device_id_constraint(device_id));
    }
    audio
}

fn browser_voice_processing_enabled() -> bool {
    !browser_media_diagnostics_enabled()
}

fn exact_device_id_constraint(device_id: &str) -> JsValue {
    let exact = Object::new();
    set_property(&exact, "exact", &JsValue::from_str(device_id));
    exact.into()
}

struct TrackSettingsSummary {
    channel_count: Option<u32>,
    sample_rate: Option<u32>,
    echo_cancellation: Option<bool>,
    noise_suppression: Option<bool>,
    auto_gain_control: Option<bool>,
    voice_isolation: Option<bool>,
}

fn track_settings_device_id(track: &web_sys::MediaStreamTrack) -> Option<String> {
    let settings = track_settings(track)?;
    Reflect::get(&settings, &JsValue::from_str("deviceId"))
        .ok()
        .and_then(|value| value.as_string())
        .filter(|value| !value.is_empty())
}

fn track_settings_summary(track: &web_sys::MediaStreamTrack) -> TrackSettingsSummary {
    let settings = track_settings(track);
    TrackSettingsSummary {
        channel_count: settings
            .as_ref()
            .and_then(|settings| optional_u32(settings, "channelCount")),
        sample_rate: settings
            .as_ref()
            .and_then(|settings| optional_u32(settings, "sampleRate")),
        echo_cancellation: settings
            .as_ref()
            .and_then(|settings| optional_bool(settings, "echoCancellation")),
        noise_suppression: settings
            .as_ref()
            .and_then(|settings| optional_bool(settings, "noiseSuppression")),
        auto_gain_control: settings
            .as_ref()
            .and_then(|settings| optional_bool(settings, "autoGainControl")),
        voice_isolation: settings
            .as_ref()
            .and_then(|settings| optional_bool(settings, "voiceIsolation")),
    }
}

fn track_settings(track: &web_sys::MediaStreamTrack) -> Option<JsValue> {
    let get_settings = Reflect::get(track.as_ref(), &JsValue::from_str("getSettings"))
        .ok()?
        .dyn_into::<Function>()
        .ok()?;
    get_settings.call0(track.as_ref()).ok()
}

fn optional_bool(settings: &JsValue, name: &str) -> Option<bool> {
    Reflect::get(settings, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_bool())
}

fn optional_u32(settings: &JsValue, name: &str) -> Option<u32> {
    Reflect::get(settings, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| value.min(f64::from(u32::MAX)) as u32)
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
