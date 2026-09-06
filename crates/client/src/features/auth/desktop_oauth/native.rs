//! Выбор платформы для передачи результата desktop OAuth.

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
#[path = "desktop.rs"]
mod platform;
#[cfg(not(all(feature = "desktop", not(target_arch = "wasm32"))))]
#[path = "unsupported.rs"]
mod platform;

pub(super) use platform::Platform;
