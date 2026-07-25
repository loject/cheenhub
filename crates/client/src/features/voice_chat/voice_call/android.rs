//! Android-владелец foreground service и событий audio focus голосового звонка.

use std::sync::{Mutex, OnceLock};

use futures_channel::mpsc;
use jni::JNIEnv;
use jni::objects::JObject;
use jni::sys::jint;

use crate::features::runtime::android::{
    AndroidBridgeError, ForegroundServiceKind, android_bridge,
};

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
