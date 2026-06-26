//! Получение браузерного потока камеры.

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use dioxus::prelude::debug;
use js_sys::{Function, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{MediaStream, MediaStreamConstraints, MediaTrackConstraints, window};

use super::backend::{CameraConfig, CameraError, CameraErrorKind};
use super::browser_errors::{is_permission_denied_error, js_error_message};

pub(super) async fn request_camera_stream(
    config: &CameraConfig,
) -> Result<MediaStream, CameraError> {
    let window = window().ok_or_else(|| CameraError::new("Окно браузера недоступно."))?;
    let media_devices = window
        .navigator()
        .media_devices()
        .map_err(|error| CameraError::new(js_error_message(error)))?;
    let video = camera_track_constraints(config);

    let constraints = MediaStreamConstraints::new();
    constraints.set_audio(&JsValue::FALSE);
    constraints.set_video(&video);

    let promise = media_devices
        .get_user_media_with_constraints(&constraints)
        .map_err(camera_error)?;
    let stream = JsFuture::from(promise).await.map_err(camera_error)?;

    stream
        .dyn_into::<MediaStream>()
        .map_err(|_| CameraError::new("Браузер вернул некорректный поток камеры."))
}

pub(super) fn first_video_track(
    stream: &MediaStream,
) -> Result<web_sys::MediaStreamTrack, CameraError> {
    let tracks = stream.get_video_tracks();
    let Some(track) = tracks.get(0).dyn_into::<web_sys::MediaStreamTrack>().ok() else {
        return Err(CameraError::new("Камера не вернула видеодорожку."));
    };

    Ok(track)
}

pub(super) fn log_selected_video_track(track: &web_sys::MediaStreamTrack) {
    let settings = video_track_settings(track);
    debug!(
        width = settings.width,
        height = settings.height,
        frame_rate = settings.frame_rate,
        "browser camera capture track selected"
    );
}

pub(super) fn video_track_settings(track: &web_sys::MediaStreamTrack) -> VideoTrackSettings {
    let Some(settings) = Reflect::get(track.as_ref(), &JsValue::from_str("getSettings"))
        .ok()
        .and_then(|get_settings| get_settings.dyn_into::<Function>().ok())
        .and_then(|get_settings| get_settings.call0(track.as_ref()).ok())
    else {
        return VideoTrackSettings::default();
    };

    VideoTrackSettings {
        width: number_setting(&settings, "width"),
        height: number_setting(&settings, "height"),
        frame_rate: number_setting(&settings, "frameRate"),
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct VideoTrackSettings {
    pub(super) width: Option<u32>,
    pub(super) height: Option<u32>,
    pub(super) frame_rate: Option<u32>,
}

fn camera_track_constraints(config: &CameraConfig) -> MediaTrackConstraints {
    let video = MediaTrackConstraints::new();
    video.set_width(&JsValue::from_f64(f64::from(config.width)));
    video.set_height(&JsValue::from_f64(f64::from(config.height)));
    video.set_frame_rate(&JsValue::from_f64(f64::from(config.frame_rate)));
    video
}

fn number_setting(settings: &JsValue, name: &str) -> Option<u32> {
    Reflect::get(settings, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.round() as u32)
}

fn camera_error(error: JsValue) -> CameraError {
    if is_permission_denied_error(&error) {
        CameraError::with_kind(
            "Доступ к камере запрещен.",
            CameraErrorKind::PermissionDenied,
        )
    } else {
        CameraError::new(js_error_message(error))
    }
}
