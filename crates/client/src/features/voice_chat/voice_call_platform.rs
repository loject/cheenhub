//! Выбор платформенного владельца системного voice-call lifecycle.

/// Системный маршрут вывода звука голосового звонка.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VoiceOutputRoute {
    /// Тихий режим через разговорный динамик.
    Earpiece,
    /// Громкая связь через основной динамик.
    Speaker,
}

#[cfg(target_os = "android")]
#[path = "voice_call/android.rs"]
mod implementation;

#[cfg(not(target_os = "android"))]
#[path = "voice_call/unsupported.rs"]
mod implementation;

pub(crate) use implementation::{
    VoiceAudioFocusEvent, load_voice_output_route, set_voice_call_participating,
    set_voice_output_route, subscribe_voice_audio_focus, supports_voice_output_route,
};
