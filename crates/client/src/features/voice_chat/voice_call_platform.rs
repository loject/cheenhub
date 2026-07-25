//! Выбор платформенного владельца системного voice-call lifecycle.

#[cfg(target_os = "android")]
#[path = "voice_call/android.rs"]
mod implementation;

#[cfg(not(target_os = "android"))]
#[path = "voice_call/unsupported.rs"]
mod implementation;

pub(crate) use implementation::{
    VoiceAudioFocusEvent, set_voice_call_participating, subscribe_voice_audio_focus,
};
