//! Выбор платформенной реализации системного трея.

#[cfg(all(
    feature = "system-tray",
    feature = "desktop",
    any(feature = "windows", feature = "linux"),
    not(target_arch = "wasm32")
))]
#[path = "desktop.rs"]
mod platform;

#[cfg(not(all(
    feature = "system-tray",
    feature = "desktop",
    any(feature = "windows", feature = "linux"),
    not(target_arch = "wasm32")
)))]
#[path = "unsupported.rs"]
mod platform;

pub(crate) use platform::{SystemTrayPlatformEffects, is_supported};
