//! Android-реализация воспроизведения аудио через `cpal`.

#[path = "cpal_playback.rs"]
mod cpal_playback;

pub(crate) use cpal_playback::{AudioPlaybackHandle, AudioPlaybackProvider};
