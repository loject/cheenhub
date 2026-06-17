//! Получение потока браузерной демонстрации экрана.

use dioxus::prelude::debug;
use js_sys::{Function, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{DisplayMediaStreamConstraints, MediaStream, window};

use super::backend::ScreenShareError;
use super::browser_errors::{is_permission_denied_error, js_error_message};

pub(super) async fn request_screen_stream() -> Result<MediaStream, ScreenShareError> {
    let window = window().ok_or_else(|| ScreenShareError::new("Окно браузера недоступно."))?;
    let media_devices = window
        .navigator()
        .media_devices()
        .map_err(|error| ScreenShareError::new(js_error_message(error)))?;

    let constraints = DisplayMediaStreamConstraints::new();
    constraints.set_audio(&JsValue::FALSE);
    constraints.set_video(&JsValue::TRUE);

    let promise = media_devices
        .get_display_media_with_constraints(&constraints)
        .map_err(screen_share_error)?;
    let stream = JsFuture::from(promise).await.map_err(screen_share_error)?;

    stream
        .dyn_into::<MediaStream>()
        .map_err(|_| ScreenShareError::new("Браузер вернул некорректный поток экрана."))
}

pub(super) fn first_video_track(
    stream: &MediaStream,
) -> Result<web_sys::MediaStreamTrack, ScreenShareError> {
    let tracks = stream.get_video_tracks();
    let Some(track) = tracks.get(0).dyn_into::<web_sys::MediaStreamTrack>().ok() else {
        return Err(ScreenShareError::new(
            "Демонстрация экрана не вернула видеодорожку.",
        ));
    };

    Ok(track)
}

pub(super) fn log_selected_video_track(track: &web_sys::MediaStreamTrack) {
    let settings = video_track_settings(track);
    debug!(
        width = settings.width,
        height = settings.height,
        frame_rate = settings.frame_rate,
        "browser screen sharing capture track selected"
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

fn number_setting(settings: &JsValue, name: &str) -> Option<u32> {
    Reflect::get(settings, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.round() as u32)
}

fn screen_share_error(error: JsValue) -> ScreenShareError {
    if is_permission_denied_error(&error) {
        ScreenShareError::permission_denied("Доступ к демонстрации экрана запрещен.")
    } else {
        ScreenShareError::new(js_error_message(error))
    }
}
