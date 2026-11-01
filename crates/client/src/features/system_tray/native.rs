//! Выбор платформенной реализации системного трея.

#[cfg(all(
    feature = "system-tray",
    feature = "desktop",
    any(feature = "windows", feature = "linux"),
    not(target_arch = "wasm32")
))]
#[path = "desktop.rs"]
mod platform;

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[path = "web.rs"]
mod platform;

#[cfg(not(any(
    all(feature = "web", target_arch = "wasm32"),
    all(
        feature = "system-tray",
        feature = "desktop",
        any(feature = "windows", feature = "linux"),
        not(target_arch = "wasm32")
    )
)))]
#[path = "unsupported.rs"]
mod platform;

pub(crate) use platform::{SystemTrayPlatformEffects, is_supported};
