//! Native-выбор способа закрытия основного окна после запуска обновления.

#[cfg(target_arch = "wasm32")]
#[path = "web.rs"]
mod platform;

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
#[path = "desktop.rs"]
mod platform;

#[cfg(all(not(feature = "desktop"), not(target_arch = "wasm32")))]
#[path = "unsupported.rs"]
mod platform;

pub(crate) use platform::use_application_update_shutdown;
