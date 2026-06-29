//! Выбор реализации воспроизведения аудио для конкретной платформы.

#[cfg(all(not(target_arch = "wasm32"), feature = "linux"))]
mod linux;
#[cfg(all(not(target_arch = "wasm32"), feature = "macos"))]
mod macos;
#[cfg(all(not(target_arch = "wasm32"), feature = "windows"))]
mod windows;

#[cfg(all(
    not(target_arch = "wasm32"),
    not(any(feature = "windows", feature = "linux", feature = "macos"))
))]
pub(crate) use super::unsupported::{AudioPlaybackHandle, AudioPlaybackProvider};
#[cfg(target_arch = "wasm32")]
pub(crate) use super::web::{AudioPlaybackHandle, AudioPlaybackProvider};
#[cfg(all(not(target_arch = "wasm32"), feature = "linux"))]
pub(crate) use linux::{AudioPlaybackHandle, AudioPlaybackProvider};
#[cfg(all(not(target_arch = "wasm32"), feature = "macos"))]
pub(crate) use macos::{AudioPlaybackHandle, AudioPlaybackProvider};
#[cfg(all(not(target_arch = "wasm32"), feature = "windows"))]
pub(crate) use windows::{AudioPlaybackHandle, AudioPlaybackProvider};
