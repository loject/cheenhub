//! Выбор платформенной реализации автоматического запуска.

#[cfg(all(feature = "windows", not(target_arch = "wasm32")))]
#[path = "windows.rs"]
mod platform;

#[cfg(any(target_arch = "wasm32", not(feature = "windows")))]
#[path = "unsupported.rs"]
mod platform;

pub(crate) use platform::{is_enabled, is_supported, set_enabled, started_hidden};
