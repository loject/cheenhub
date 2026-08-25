//! Выбор реализации WebSocket для текущей платформы.

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
pub(in crate::features::host_settings::log_stream) use super::unsupported::run;

#[cfg(target_arch = "wasm32")]
pub(in crate::features::host_settings::log_stream) use super::web::run;

#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "desktop", feature = "mobile")
))]
pub(in crate::features::host_settings::log_stream) use desktop::run;
