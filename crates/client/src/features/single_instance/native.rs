//! Выбор платформенной реализации единственного экземпляра.

#[cfg(all(feature = "windows", not(target_arch = "wasm32")))]
#[path = "windows.rs"]
mod platform;

#[cfg(any(target_arch = "wasm32", not(feature = "windows")))]
#[path = "unsupported.rs"]
mod platform;

pub(crate) use platform::SingleInstanceEffects;
pub(crate) use platform::prepare;
