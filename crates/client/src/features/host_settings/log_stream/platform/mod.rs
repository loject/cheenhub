//! Выбор платформенной реализации потока логов.

mod native;
#[cfg(all(
    not(target_arch = "wasm32"),
    not(any(feature = "desktop", feature = "mobile"))
))]
mod unsupported;
#[cfg(target_arch = "wasm32")]
mod web;

pub(super) use native::run;
