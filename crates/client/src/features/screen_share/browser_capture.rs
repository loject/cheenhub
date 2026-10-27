//! Получение потока браузерной демонстрации экрана.

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]

use dioxus::prelude::debug;
use js_sys::{Function, Object, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{DisplayMediaStreamConstraints, MediaStream, window};

use super::backend::{ScreenShareError, ScreenShareErrorKind};
use super::browser_errors::{is_permission_denied_error, js_error_message};

#[cfg(test)]
mod tests;

pub(super) async fn request_screen_stream() -> Result<MediaStream, ScreenShareError> {
    let window = window().ok_or_else(|| ScreenShareError::new("Окно браузера недоступно."))?;
    let media_devices = window
        .navigator()
        .media_devices()
        .map_err(|error| ScreenShareError::new(js_error_message(error)))?;

    let constraints = DisplayMediaStreamConstraints::new();
    let audio = Object::new();
    for name in ["echoCancellation", "noiseSuppression", "autoGainControl"] {
        Reflect::set(&audio, &JsValue::from_str(name), &JsValue::FALSE)
            .map_err(screen_share_error)?;
    }
    Reflect::set(
        &audio,
        &JsValue::from_str("restrictOwnAudio"),
        &JsValue::TRUE,
    )
    .map_err(screen_share_error)?;
    constraints.set_audio(&audio);
    constraints.set_video(&JsValue::TRUE);
    for (name, value) in [
        ("systemAudio", "include"),
        ("windowAudio", "exclude"),
        ("surfaceSwitching", "exclude"),
        ("selfBrowserSurface", "exclude"),
    ] {
        Reflect::set(
            constraints.as_ref(),
            &JsValue::from_str(name),
            &JsValue::from_str(value),
        )
        .map_err(screen_share_error)?;
    }

    let promise = media_devices
        .get_display_media_with_constraints(&constraints)
        .map_err(screen_share_error)?;
    let stream = JsFuture::from(promise).await.map_err(screen_share_error)?;

    stream
        .dyn_into::<MediaStream>()
        .map_err(|_| ScreenShareError::new("Браузер вернул некорректный поток экрана."))
}

/// Останавливает все дорожки потока, включая аудио незавершённого запуска.
pub(super) fn stop_screen_stream(stream: &MediaStream) {
    let tracks = stream.get_tracks();
    for index in 0..tracks.length() {
        if let Ok(track) = tracks.get(index).dyn_into::<web_sys::MediaStreamTrack>() {
            track.stop();
        }
    }
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
    let Some(settings) = track_settings(track) else {
        return VideoTrackSettings::default();
    };

    VideoTrackSettings {
        width: number_setting(&settings, "width"),
        height: number_setting(&settings, "height"),
        frame_rate: number_setting(&settings, "frameRate"),
        display_surface: Reflect::get(&settings, &JsValue::from_str("displaySurface"))
            .ok()
            .and_then(|value| value.as_string()),
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct VideoTrackSettings {
    pub(super) width: Option<u32>,
    pub(super) height: Option<u32>,
    pub(super) frame_rate: Option<u32>,
    /// Подтверждённый браузером тип выбранного источника.
    pub(super) display_surface: Option<String>,
}

fn track_settings(track: &web_sys::MediaStreamTrack) -> Option<JsValue> {
    Reflect::get(track.as_ref(), &JsValue::from_str("getSettings"))
        .ok()?
        .dyn_into::<Function>()
        .ok()?
        .call0(track.as_ref())
        .ok()
}

fn system_audio_allowed(display_surface: Option<&str>) -> bool {
    display_surface == Some("monitor")
}

/// Проверяет фактический источник; неизвестные настройки запрещают системный звук.
pub(super) fn whole_screen_selected(video: &web_sys::MediaStreamTrack) -> bool {
    system_audio_allowed(video_track_settings(video).display_surface.as_deref())
}

/// Оставляет одну аудиодорожку только для целого экрана, останавливая остальные.
pub(super) fn selected_system_audio(
    stream: &MediaStream,
    video: &web_sys::MediaStreamTrack,
) -> Option<web_sys::MediaStreamTrack> {
    let tracks = stream.get_audio_tracks();
    let allowed = whole_screen_selected(video);
    let mut selected = None;
    for index in 0..tracks.length() {
        if let Ok(track) = tracks.get(index).dyn_into::<web_sys::MediaStreamTrack>() {
            if allowed && selected.is_none() {
                selected = Some(track);
            } else {
                track.stop();
            }
        }
    }
    debug!(
        allowed,
        has_audio = selected.is_some(),
        "selected screen sharing audio capture policy"
    );
    selected
}

/// Проверяет подтверждённую браузером настройку исключения звука страницы.
pub(super) fn own_audio_restricted(track: &web_sys::MediaStreamTrack) -> bool {
    track_settings(track)
        .and_then(|settings| Reflect::get(&settings, &JsValue::from_str("restrictOwnAudio")).ok())
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
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
        ScreenShareError::with_kind(
            "Доступ к демонстрации экрана запрещен.",
            ScreenShareErrorKind::PermissionDenied,
        )
    } else {
        ScreenShareError::new(js_error_message(error))
    }
}
