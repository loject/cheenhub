//! Android-реализация воспроизведения аудио с профилем голосовой связи.

#[path = "cpal_playback.rs"]
mod cpal_playback;
#[path = "android_engine.rs"]
mod platform_engine;

pub(crate) use cpal_playback::{AudioPlaybackHandle, AudioPlaybackProvider};
