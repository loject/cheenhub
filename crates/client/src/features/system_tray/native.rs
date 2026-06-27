//! Выбор платформенной реализации системного трея.

#[cfg(all(
    feature = "system-tray",
    feature = "desktop",
    not(target_arch = "wasm32")
))]
#[path = "desktop.rs"]
mod platform;

#[cfg(any(
    target_arch = "wasm32",
    not(feature = "system-tray"),
    not(feature = "desktop")
))]
#[path = "unsupported.rs"]
mod platform;

pub(crate) use platform::SystemTrayPlatformEffects;
