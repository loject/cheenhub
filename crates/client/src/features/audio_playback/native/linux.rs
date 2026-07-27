//! Linux-реализация воспроизведения аудио через `cpal`.

#[path = "cpal_playback.rs"]
mod cpal_playback;
#[path = "engine.rs"]
mod platform_engine;

pub(crate) use cpal_playback::{AudioPlaybackHandle, AudioPlaybackProvider};
