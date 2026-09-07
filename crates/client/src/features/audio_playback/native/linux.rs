//! Linux-реализация воспроизведения аудио через PulseAudio API.

#[path = "cpal_playback.rs"]
mod cpal_playback;
#[path = "linux_engine.rs"]
mod platform_engine;

pub(crate) use cpal_playback::{AudioPlaybackHandle, AudioPlaybackProvider};
