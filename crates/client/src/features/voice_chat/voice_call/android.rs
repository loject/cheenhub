//! Android-владелец foreground service и событий audio focus голосового звонка.

use std::sync::{Mutex, OnceLock};

use dioxus::logger::tracing::{info, warn};
use futures_channel::mpsc;
use futures_channel::oneshot;
use jni::JNIEnv;
use jni::objects::{JObject, JValue};
use jni::sys::jint;

use crate::features::runtime::android::{
    AndroidBridgeError, ForegroundServiceKind, android_bridge,
};

use super::VoiceOutputRoute;

/// Изменение доступности Android audio focus для активного звонка.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VoiceAudioFocusEvent {
    /// Audio focus снова доступен.
    Gained,
    /// Audio focus временно потерян.
    LostTransient,
    /// Audio focus потерян без гарантии автоматического возврата.
    Lost,
}

static AUDIO_FOCUS_SUBSCRIBERS: OnceLock<Mutex<Vec<mpsc::UnboundedSender<VoiceAudioFocusEvent>>>> =
    OnceLock::new();

/// Обновляет владение Android foreground service согласно участию в звонке.
pub(crate) fn set_voice_call_participating(participating: bool) -> Result<(), AndroidBridgeError> {
    let bridge = android_bridge()?;
    if participating {
        bridge.start_foreground_service(ForegroundServiceKind::VoicePlayback)
    } else {
        bridge.stop_foreground_service(ForegroundServiceKind::VoicePlayback)
    }
}

/// Сообщает, что Android может предоставить системный переключатель маршрута звонка.
pub(crate) fn supports_voice_output_route() -> bool {
    true
}

/// Загружает доступность и текущий системный маршрут вывода звонка.
pub(crate) async fn load_voice_output_route() -> Result<Option<VoiceOutputRoute>, AndroidBridgeError>
{
    let (sender, receiver) = oneshot::channel();
    wry::prelude::dispatch(move |env, activity, _| {
        let result = env
            .call_method(activity, "getCheenHubVoiceOutputRoute", "()I", &[])
            .and_then(|value| value.i())
            .map_err(|error| {
                AndroidBridgeError::new(format!(
                    "Не удалось получить Android-маршрут вывода звонка: {error}"
                ))
            })
            .and_then(|route| match route {
                0 => Ok(None),
                1 => Ok(Some(VoiceOutputRoute::Earpiece)),
                2 => Ok(Some(VoiceOutputRoute::Speaker)),
                route => Err(AndroidBridgeError::new(format!(
                    "Android вернул неизвестный маршрут вывода звонка: {route}"
                ))),
            });
        let _ = sender.send(result);
    });
    let result = receiver.await.map_err(|_| {
        AndroidBridgeError::new("Android не завершил чтение маршрута вывода звонка")
    })?;
    match &result {
        Ok(Some(route)) => info!(?route, "Android voice output route loaded"),
        Ok(None) => info!("Android voice output route switch is unavailable"),
        Err(error) => warn!(%error, "failed to load Android voice output route"),
    }
    result
}

/// Переключает системный маршрут вывода активного звонка.
pub(crate) async fn set_voice_output_route(
    route: VoiceOutputRoute,
) -> Result<(), AndroidBridgeError> {
    let speaker = route == VoiceOutputRoute::Speaker;
    let (sender, receiver) = oneshot::channel();
    wry::prelude::dispatch(move |env, activity, _| {
        let result = env
            .call_method(
                activity,
                "setCheenHubVoiceOutputRoute",
                "(Z)Z",
                &[JValue::Bool(speaker.into())],
            )
            .and_then(|value| value.z())
            .map_err(|error| {
                AndroidBridgeError::new(format!(
                    "Не удалось переключить Android-маршрут вывода звонка: {error}"
                ))
            })
            .and_then(|changed| {
                if changed {
                    Ok(())
                } else {
                    Err(AndroidBridgeError::new(
                        "Запрошенный Android-маршрут вывода звонка недоступен",
                    ))
                }
            });
        let _ = sender.send(result);
    });
    let result = receiver.await.map_err(|_| {
        AndroidBridgeError::new("Android не завершил переключение маршрута вывода звонка")
    })?;
    match &result {
        Ok(()) => info!(?route, "Android voice output route changed"),
        Err(error) => warn!(%error, ?route, "failed to change Android voice output route"),
    }
    result
}

/// Подписывается на изменения Android audio focus.
pub(crate) fn subscribe_voice_audio_focus() -> mpsc::UnboundedReceiver<VoiceAudioFocusEvent> {
    let (sender, receiver) = mpsc::unbounded();
    if let Ok(mut subscribers) = audio_focus_subscribers().lock() {
        subscribers.push(sender);
    }
    receiver
}

fn audio_focus_subscribers() -> &'static Mutex<Vec<mpsc::UnboundedSender<VoiceAudioFocusEvent>>> {
    AUDIO_FOCUS_SUBSCRIBERS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Передаёт изменение Android audio focus активному voice provider.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_dioxus_main_MainActivity_nativeOnCheenHubVoiceAudioFocusChanged(
    _env: JNIEnv<'_>,
    _activity: JObject<'_>,
    focus_change: jint,
) {
    let event = match focus_change {
        1 => VoiceAudioFocusEvent::Gained,
        -2 | -3 => VoiceAudioFocusEvent::LostTransient,
        -1 => VoiceAudioFocusEvent::Lost,
        _ => return,
    };
    let Ok(mut subscribers) = audio_focus_subscribers().lock() else {
        return;
    };
    subscribers.retain(|subscriber| subscriber.unbounded_send(event).is_ok());
}
