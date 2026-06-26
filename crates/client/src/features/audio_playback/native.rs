//! Выбор платформенной реализации воспроизведения аудио.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use super::unsupported::{AudioPlaybackHandle, AudioPlaybackProvider};
#[cfg(target_arch = "wasm32")]
pub(crate) use super::web::{AudioPlaybackHandle, AudioPlaybackProvider};
