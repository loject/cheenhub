//! Выбор платформенной реализации воспроизведения аудио.

#[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
#[path = "native/cpal_playback.rs"]
mod cpal_playback;

#[cfg(all(not(target_arch = "wasm32"), not(feature = "native-audio")))]
pub(crate) use super::unsupported::{AudioPlaybackHandle, AudioPlaybackProvider};
#[cfg(target_arch = "wasm32")]
pub(crate) use super::web::{AudioPlaybackHandle, AudioPlaybackProvider};
#[cfg(all(not(target_arch = "wasm32"), feature = "native-audio"))]
pub(crate) use cpal_playback::{AudioPlaybackHandle, AudioPlaybackProvider};
