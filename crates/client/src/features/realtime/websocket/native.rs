//! Выбор платформенной реализации WebSocket fallback.

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "desktop", feature = "mobile")
))]
#[path = "desktop.rs"]
mod desktop;

#[cfg(all(
    not(target_arch = "wasm32"),
    not(any(feature = "desktop", feature = "mobile"))
))]
pub(in crate::features::realtime) use super::unsupported::{spawn_reader, spawn_writer, split};
#[cfg(target_arch = "wasm32")]
pub(in crate::features::realtime) use super::web::{spawn_reader, spawn_writer, split};
#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "desktop", feature = "mobile")
))]
pub(in crate::features::realtime) use desktop::{spawn_reader, spawn_writer, split};
