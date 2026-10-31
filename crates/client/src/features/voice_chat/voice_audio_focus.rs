//! Подписка на события аудиофокуса активного голосового звонка.

use dioxus::prelude::*;
use futures_util::StreamExt;

use super::voice_call_platform::{self, VoiceAudioFocusEvent};

/// Передаёт изменения Android audio focus владельцу медиасостояния.
pub(super) fn use_voice_audio_focus(mut focused: Signal<bool>) {
    use_hook(move || {
        spawn(async move {
            let mut events = voice_call_platform::subscribe_voice_audio_focus();
            while let Some(event) = events.next().await {
                focused.set(matches!(event, VoiceAudioFocusEvent::Gained));
                match event {
                    VoiceAudioFocusEvent::Gained => info!("Android voice-call audio focus gained"),
                    VoiceAudioFocusEvent::LostTransient => {
                        warn!("Android voice-call audio focus lost temporarily");
                    }
                    VoiceAudioFocusEvent::Lost => warn!("Android voice-call audio focus lost"),
                }
            }
        })
    });
}
